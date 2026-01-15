use prost_types::Timestamp;
use std::time::SystemTime;
use tokio::{fs::File, io::AsyncReadExt};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};

use grpc_files::fileservice::{
    CreateDirectoryRequest, CreateDirectoryResponse, DeleteDirectoryRequest, DeleteDirectoryResponse,
    DeleteRequest, DeleteResponse, DownloadChunk, DownloadRequest, FileInfo, ListRequest,
    ListResponse, UploadChunk, UploadResponse,
    file_service_server::{FileService, FileServiceServer},
};

#[derive(Clone)]
struct GRPCFileStore {
    storage_path: String,
}

impl GRPCFileStore {
    pub fn new(storage_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&storage_path)?;
        Ok(GRPCFileStore { storage_path })
    }

    /// Resolve a relative path to an absolute path within storage, preventing directory traversal.
    fn resolve_path(&self, relative_path: &str) -> Result<String, tonic::Status> {
        let clean_path = relative_path.trim_start_matches('/').trim_end_matches('/');

        // Block path traversal attempts
        if clean_path.contains("..") {
            return Err(tonic::Status::invalid_argument("Path traversal not allowed"));
        }

        if clean_path.is_empty() {
            Ok(self.storage_path.clone())
        } else {
            Ok(format!("{}/{}", self.storage_path, clean_path))
        }
    }

    /// Check if a path exists and is a directory.
    async fn ensure_directory_exists(&self, path: &str) -> Result<(), tonic::Status> {
        let path = std::path::Path::new(path);
        if path.exists() {
            if path.is_dir() {
                Ok(())
            } else {
                Err(tonic::Status::failed_precondition("Path exists but is not a directory"))
            }
        } else {
            Err(tonic::Status::not_found("Directory does not exist"))
        }
    }
}

#[tonic::async_trait]
impl FileService for GRPCFileStore {
    async fn upload(
        &self,
        request: tonic::Request<tonic::Streaming<UploadChunk>>,
    ) -> Result<tonic::Response<UploadResponse>, tonic::Status> {
        let mut stream = request.into_inner();
        let first_chunk = stream.message().await?.unwrap();
        let filename = first_chunk.filename;
        let upload_id = first_chunk.upload_id;

        // Handle target directory from first chunk
        let target_dir = if !first_chunk.target_directory.is_empty() {
            self.resolve_path(&first_chunk.target_directory)?
        } else {
            self.storage_path.clone()
        };

        // Ensure target directory exists
        self.ensure_directory_exists(&target_dir).await?;

        let temp_path = format!("{}/{}.tmp", target_dir, upload_id);
        let mut file = tokio::fs::File::create(&temp_path)
            .await
            .map_err(|e| tonic::Status::internal(format!("Failed to create file: {}", e)))?;

        let mut total_size = first_chunk.data.len() as u64;

        // write first chunk
        tokio::io::AsyncWriteExt::write_all(&mut file, &first_chunk.data).await?;

        // write the rest of the chunks
        while let Some(chunk) = stream.message().await? {
            total_size += chunk.data.len() as u64;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk.data).await?;
        }

        let final_path = format!("{}/{}", target_dir, filename);
        tokio::fs::rename(&temp_path, &final_path).await?;

        Ok(tonic::Response::new(UploadResponse {
            file_id: upload_id,
            filename,
            size: total_size,
            upload_time: Some(Timestamp::from(SystemTime::now())),
        }))
    }

    type DownloadStream = ReceiverStream<Result<DownloadChunk, tonic::Status>>;

    async fn download(
        &self,
        request: tonic::Request<DownloadRequest>,
    ) -> Result<tonic::Response<Self::DownloadStream>, tonic::Status> {
        let filename = request.into_inner().file_name;
        let full_path = self.storage_path.clone() + "/" + filename.as_str();

        let (tx, rx) = tokio::sync::mpsc::channel(32);

        tokio::spawn(async move {
            let mut file = File::open(full_path).await.unwrap();
            let mut buffer = vec![0u8; 1024 * 1024];
            loop {
                match file.read(&mut buffer[..]).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = DownloadChunk {
                            data: buffer[..n].to_vec(),
                        };

                        if tx.send(Ok(chunk)).await.is_err() {
                            break;
                        }
                    }
                    Err(_e) => break,
                }
            }
        });

        let res = ReceiverStream::new(rx);

        Ok(tonic::Response::new(res))
    }

    async fn delete_file(
        &self,
        request: tonic::Request<DeleteRequest>,
    ) -> Result<tonic::Response<DeleteResponse>, tonic::Status> {
        let req = request.into_inner();
        let full_path = self.resolve_path(&req.file_name)?;
        println!("{}", full_path);
        tokio::fs::remove_file(&full_path)
            .await
            .map_err(|e| tonic::Status::not_found(e.to_string()))?;
        Ok(tonic::Response::new(DeleteResponse {}))
    }

    async fn list_files(
        &self,
        request: tonic::Request<ListRequest>,
    ) -> Result<tonic::Response<ListResponse>, tonic::Status> {
        let req = request.into_inner();
        let request_path = req.path;
        let full_path = self.resolve_path(&request_path)?;

        // Ensure the path exists and is a directory
        self.ensure_directory_exists(&full_path).await?;

        let mut entries = tokio::fs::read_dir(&full_path)
            .await
            .map_err(|e| tonic::Status::internal(format!("Failed to read directory: {}", e)))?;

        let mut items: Vec<FileInfo> = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let filename = entry.file_name()
                .into_string()
                .unwrap_or_default();

            // Skip hidden files/directories
            if filename.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata().await.unwrap();
            let is_dir = metadata.is_dir();

            // For directories, use 0 size
            let size = if is_dir { 0 } else { metadata.len() };

            // Get creation time (use modified for directories as fallback)
            let created = if is_dir {
                metadata.modified().ok()
            } else {
                metadata.created().ok()
            };
            let upload_time = created.map(Timestamp::from);

            // Build relative path for this item
            let item_path = if request_path.is_empty() {
                filename.clone()
            } else {
                format!("{}/{}", request_path, filename)
            };

            items.push(FileInfo {
                filename,
                size,
                upload_time,
                is_directory: is_dir,
                path: item_path,
            });
        }

        // Sort: directories first, then alphabetically
        items.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.filename.cmp(&b.filename),
            }
        });

        Ok(tonic::Response::new(ListResponse {
            files: items,
            current_path: request_path,
        }))
    }

    async fn create_directory(
        &self,
        request: tonic::Request<CreateDirectoryRequest>,
    ) -> Result<tonic::Response<CreateDirectoryResponse>, tonic::Status> {
        let req = request.into_inner();

        // Resolve the parent path
        let parent_path = self.resolve_path(&req.path)?;
        let dir_name = req.name.trim().trim_end_matches('/');

        // Validate directory name
        if dir_name.is_empty() || dir_name.contains('/') {
            return Err(tonic::Status::invalid_argument("Invalid directory name"));
        }

        let full_path = format!("{}/{}", parent_path, dir_name);

        // Check if already exists
        if std::path::Path::new(&full_path).exists() {
            return Err(tonic::Status::already_exists("Directory already exists"));
        }

        // Create the directory
        tokio::fs::create_dir(&full_path)
            .await
            .map_err(|e| tonic::Status::internal(format!("Failed to create directory: {}", e)))?;

        Ok(tonic::Response::new(CreateDirectoryResponse {}))
    }

    async fn delete_directory(
        &self,
        request: tonic::Request<DeleteDirectoryRequest>,
    ) -> Result<tonic::Response<DeleteDirectoryResponse>, tonic::Status> {
        let req = request.into_inner();
        let full_path = self.resolve_path(&req.path)?;
        let path = std::path::Path::new(&full_path);

        // Verify path exists and is a directory
        if !path.exists() {
            return Err(tonic::Status::not_found("Directory not found"));
        }

        if !path.is_dir() {
            return Err(tonic::Status::failed_precondition("Path is not a directory"));
        }

        // Check if directory is empty
        let mut entries = tokio::fs::read_dir(&full_path).await.unwrap();
        let mut is_empty = true;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') {
                is_empty = false;
                break;
            }
        }

        if !is_empty && !req.recursive {
            return Err(tonic::Status::failed_precondition(
                "Directory is not empty. Use recursive=true to delete."
            ));
        }

        // Delete the directory
        if req.recursive {
            tokio::fs::remove_dir_all(&full_path)
                .await
                .map_err(|e| tonic::Status::internal(format!("Failed to delete directory: {}", e)))?;
        } else {
            tokio::fs::remove_dir(&full_path)
                .await
                .map_err(|e| tonic::Status::internal(format!("Failed to delete directory: {}", e)))?;
        }

        Ok(tonic::Response::new(DeleteDirectoryResponse {}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = grpc_files::config::Config::load()?;
    let auth_dir = grpc_files::config::Config::get_auth_dir()?;

    let cert = tokio::fs::read_to_string(auth_dir.join("server-cert.pem")).await?;
    let key = tokio::fs::read_to_string(auth_dir.join("server-key.pem")).await?;
    let server_identity = Identity::from_pem(cert, key);

    let client_ca_cert = tokio::fs::read_to_string(auth_dir.join("ca-cert.pem")).await?;
    let client_ca_cert = Certificate::from_pem(client_ca_cert);

    let tls = ServerTlsConfig::new()
        .identity(server_identity)
        .client_ca_root(client_ca_cert);

    let addr = config.server_bind_address.parse()?;
    let service = GRPCFileStore::new(config.upload_directory).unwrap();
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(grpc_files::fileservice::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    Server::builder()
        .tls_config(tls)?
        .initial_connection_window_size(1024 * 1024)
        .initial_stream_window_size(1024 * 1024)
        .add_service(FileServiceServer::new(service))
        .add_service(reflection)
        .serve(addr)
        .await?;

    Ok(())
}
