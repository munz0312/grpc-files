use std::error::Error;

use tokio::{fs::File, io::AsyncReadExt};

use fileservice::file_service_client::FileServiceClient;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;

use crate::fileservice::UploadChunk;
pub mod fileservice {
    tonic::include_proto!("fileservice");
}

async fn upload_file(client: &mut FileServiceClient<Channel>) -> Result<(), Box<dyn Error>> {
    let upload_id = uuid::Uuid::new_v4().to_string();
    let filename = "test_file.txt";
    let filepath = "test_file.txt";
    let mut chunk_index: u64 = 0;
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    tokio::spawn(async move {
        let mut f = File::open(filepath).await.unwrap();
        let mut buffer = vec![0u8; 8192];
        loop {
            match f.read(&mut buffer[..]).await {
                Ok(0) => {
                    // EOF
                    break;
                }
                Ok(n) => {
                    let upload_req = UploadChunk {
                        upload_id: upload_id.to_string(),
                        filename: filename.to_string(),
                        chunk_index,
                        data: buffer[..n].to_vec(),
                    };
                    if tx.send(upload_req).await.is_err() {
                        break;
                    }
                    chunk_index += 1;
                }
                Err(_e) => break,
            }
        }
    });

    let stream = ReceiverStream::new(rx);
    let response = client.upload(tonic::Request::new(stream)).await?;
    let res_data = response.into_inner();

    println!(
        "{}",
        format!(
            "File name: {}\nFile ID: {}\nSize: {} bytes\nUpload time: {}",
            res_data.filename,
            res_data.file_id,
            res_data.size,
            res_data.upload_time.unwrap().to_string()
        )
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = "http://[::1]:50051";
    let mut client = FileServiceClient::connect(url).await?;

    upload_file(&mut client).await?;

    Ok(())
}
