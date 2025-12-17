use std::{error::Error, io::Write};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use fileservice::file_service_client::FileServiceClient;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;

use crate::fileservice::{DeleteRequest, DownloadChunk, DownloadRequest, ListRequest, UploadChunk};
pub mod fileservice {
    tonic::include_proto!("fileservice");
}

async fn delete_file(
    client: &mut FileServiceClient<Channel>,
    file_name: String,
) -> Result<(), Box<dyn Error>> {
    client
        .delete_file(DeleteRequest {
            file_name: file_name.clone(),
        })
        .await?;
    println!("Deleted file: {}", file_name);
    Ok(())
}

async fn list_files(client: &mut FileServiceClient<Channel>) -> Result<(), Box<dyn Error>> {
    let files = client.list_files(ListRequest {}).await?;
    let file_data = files.into_inner();
    let file_vec = file_data.files;

    println!("All files");
    println!("Filename\tSize (Bytes)\tUpload Time");
    for file in file_vec {
        let upload_info = file.upload_time.unwrap().to_string();
        let time_info: Vec<&str> = upload_info.split('T').collect();
        let date = time_info[0];
        let time = time_info[1].split('.').next().unwrap();
        let file_info = format!(
            "{}\t{}\t\t{}",
            file.filename,
            file.size,
            date.to_string() + " " + time
        );
        println!("{}\n", file_info);
    }
    Ok(())
}

async fn download_file(
    client: &mut FileServiceClient<Channel>,
    file_name: String,
) -> Result<(), Box<dyn Error>> {
    let mut stream = client
        .download(DownloadRequest {
            file_name: file_name.clone(),
        })
        .await?
        .into_inner();
    let mut file = tokio::fs::File::create(file_name).await?;
    while let Ok(Some(chunk)) = stream.message().await {
        file.write(&chunk.data).await?;
    }
    Ok(())
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

    loop {
        print!("Enter your option: ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Error reading option");
        let choice = input.trim().parse::<u8>()?;

        match choice {
            1 => list_files(&mut client).await?,
            2 => upload_file(&mut client).await?,
            3 => {
                println!("Enter the file to download");
                let mut file_name = String::new();
                std::io::stdin().read_line(&mut file_name)?;
                download_file(&mut client, file_name).await?;
            }
            4 => {
                println!("Enter the file to upload");
                let mut file_name = String::new();
                std::io::stdin().read_line(&mut file_name)?;
                delete_file(&mut client, file_name).await?;
            }
            _ => eprintln!("Invalid choice: {}", choice),
        }
    }
}
