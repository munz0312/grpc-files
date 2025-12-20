use prost_types::Timestamp;
use std::time::SystemTime;
use tokio::{fs::File, io::AsyncReadExt};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;

use grpc_files::fileservice::{
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

        let temp_path = format!("{}/{}.tmp", self.storage_path, upload_id);
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

        let final_path = format!("{}/{}", self.storage_path, filename);
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
            let mut buffer = vec![0u8; 8192];
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
        let filename = request.into_inner().file_name;
        let full_path = self.storage_path.clone() + "/" + filename.as_str();
        println!("{}", full_path);
        tokio::fs::remove_file(full_path)
            .await
            .map_err(|e| tonic::Status::not_found(e.to_string()))?;
        Ok(tonic::Response::new(DeleteResponse {}))
    }

    async fn list_files(
        &self,
        _request: tonic::Request<ListRequest>,
    ) -> Result<tonic::Response<ListResponse>, tonic::Status> {
        let mut files = tokio::fs::read_dir(&self.storage_path).await.unwrap();
        let mut file_metadata: Vec<FileInfo> = Vec::new();

        while let Ok(Some(entry)) = files.next_entry().await {
            let filename = entry.file_name().into_string().unwrap();
            let metadata = entry.metadata().await.unwrap();
            let size = metadata.len();
            let upload_time = Timestamp::from(metadata.created().unwrap());
            file_metadata.push(FileInfo {
                filename,
                size,
                upload_time: Some(upload_time),
            });
        }

        Ok(tonic::Response::new(ListResponse {
            files: file_metadata,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let service = GRPCFileStore::new("./uploads".to_string()).unwrap();
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(grpc_files::fileservice::FILE_DESCRIPTOR_SET)
        .build_v1()?;
    Server::builder()
        .add_service(FileServiceServer::new(service))
        .add_service(reflection)
        .serve(addr)
        .await?;
    Ok(())
}
