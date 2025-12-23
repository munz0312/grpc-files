use ratatui::{
    Terminal,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{
            Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
            enable_raw_mode,
        },
    },
    prelude::{Backend, CrosstermBackend},
};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use uuid;

use crate::{
    fileservice::{
        DeleteRequest, DownloadRequest, ListRequest, UploadChunk,
        file_service_client::FileServiceClient,
    },
    tui::{
        app::{App, AppMode},
        ui::ui,
    },
};
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    let client_cert = tokio::fs::read_to_string("./auth/client-cert.pem").await?;
    let client_key = tokio::fs::read_to_string("./auth/client-key.pem").await?;
    let client_identity = Identity::from_pem(client_cert, client_key);

    let server_ca_cert = tokio::fs::read_to_string("./auth/ca-cert.pem").await?;
    let server_ca_cert = Certificate::from_pem(server_ca_cert);

    let tls = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(server_ca_cert)
        .identity(client_identity);

    let channel = Channel::from_static("https://192.168.1.244:50051")
        .tls_config(tls)?
        .connect()
        .await?;

    let mut client = FileServiceClient::new(channel);
    let res = run_app(&mut terminal, &mut app, &mut client).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res?;
    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    client: &mut FileServiceClient<Channel>,
) -> io::Result<()> {
    // Initial refresh
    if let Err(e) = refresh_files(app, client).await {
        app.set_status(format!("Error loading files: {}", e));
    }

    loop {
        terminal.draw(|f| ui(f, app))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Char('j') => app.select_next(),
                KeyCode::Char('k') => app.select_prev(),
                KeyCode::Char('r') => {
                    app.set_status("Refreshing file list...".to_string());
                    //terminal.draw(|f| ui(f, app))?;
                    if let Err(e) = refresh_files(app, client).await {
                        app.set_status(format!("Error: {}", e));
                    } else {
                        app.set_status("File list refreshed".to_string());
                    }
                }
                KeyCode::Char('X') => {
                    if let Some(file) = app.selected_file() {
                        let filename = file.filename.clone();
                        app.set_status(format!("Deleting {}...", filename));
                        //terminal.draw(|f| ui(f, app))?;
                        if let Err(e) = delete_file(client, &filename).await {
                            app.set_status(format!("Error deleting {}: {}", filename, e));
                        } else {
                            app.set_status(format!("Deleted {}", filename));
                            // Refresh after deletion
                            let _ = refresh_files(app, client).await;
                        }
                    }
                }
                KeyCode::Char('d') => {
                    if let Some(file) = app.selected_file() {
                        let filename = file.filename.clone();
                        app.set_status(format!("Downloading {}...", filename));
                        //terminal.draw(|f| ui(f, app))?;
                        if let Err(e) = download_file(client, &filename).await {
                            app.set_status(format!("Error downloading {}: {}", filename, e));
                        } else {
                            app.set_status(format!("Downloaded {}", filename));
                        }
                    }
                }
                KeyCode::Char('U') => {
                    app.set_mode(AppMode::Uploading);
                    terminal.draw(|f| ui(f, app))?;

                    prepare_terminal_for_file_selection();

                    match select_file_with_picker().await {
                        Some(path) => {
                            restore_terminal_after_file_selection();

                            app.set_mode(AppMode::Normal);
                            app.set_file_for_upload(path.clone());
                            app.set_status(format!("Uploading {}...", path));
                            if let Err(e) = upload_selected_file(client, &path).await {
                                app.set_status(format!("Upload failed: {}", e));
                            } else {
                                app.set_status("Upload completed".to_string());
                                if let Err(e) = refresh_files(app, client).await {
                                    app.set_status(format!("Error refreshing files: {}", e));
                                }
                            }
                            app.clear_file_path();
                        }
                        None => {
                            // Restore terminal even if cancelled
                            restore_terminal_after_file_selection();
                            app.set_mode(AppMode::Normal);
                            app.set_status("File selection cancelled".to_string());
                        }
                    }
                }
                KeyCode::Char('q') => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

async fn refresh_files(
    app: &mut App,
    client: &mut FileServiceClient<Channel>,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.list_files(ListRequest {}).await?;
    let files = response.into_inner().files;
    app.update_files(files);
    app.clear_status();
    Ok(())
}

async fn delete_file(
    client: &mut FileServiceClient<Channel>,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .delete_file(DeleteRequest {
            file_name: filename.to_string(),
        })
        .await?;
    Ok(())
}

async fn download_file(
    client: &mut FileServiceClient<Channel>,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = client
        .download(DownloadRequest {
            file_name: filename.to_string(),
        })
        .await?
        .into_inner();

    let full_path = format!("./downloads/{}", filename);
    let path = Path::new(&full_path);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename);

    // Check if file already exists
    if Path::new(file_name).exists() {
        return Err(format!("File '{}' already exists", file_name).into());
    }

    let mut file = File::create(path).await?;

    while let Some(chunk) = stream.next().await {
        let data = chunk?.data;
        file.write_all(&data).await?;
    }

    file.flush().await?;
    Ok(())
}

fn prepare_terminal_for_file_selection() {
    execute!(io::stdout(), DisableMouseCapture).ok();
    execute!(io::stdout(), LeaveAlternateScreen).ok();
    disable_raw_mode().ok();
    execute!(io::stdout(), Clear(ClearType::All)).ok();

    // Show a message to the user
    println!("Opening file selection dialog...");
    io::stdout().flush().ok();
}

fn restore_terminal_after_file_selection() {
    enable_raw_mode().ok();
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).ok();

    // Clear any pending input
    let _ = event::poll(Duration::from_millis(100));
    while event::poll(Duration::from_millis(0)).unwrap_or(false) {
        let _ = event::read();
    }
}

async fn select_file_with_picker() -> Option<String> {
    let result = if Command::new("zenity").output().is_ok() {
        // Try zenity (GTK)
        Command::new("zenity")
            .args(&["--file-selection", "--title=Select file to upload"])
            .output()
            .map(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or(None)
    } else if Command::new("kdialog").output().is_ok() {
        // Try kdialog (KDE)
        Command::new("kdialog")
            .args(&["--getopenfilename", "."])
            .output()
            .map(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or(None)
    } else {
        // Fallback to simple prompt
        println!("Enter file path to upload:");
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok()?;
        let path = input.trim().to_string();

        if Path::new(&path).exists() {
            Some(path)
        } else {
            None
        }
    };

    result
}

async fn upload_selected_file(
    client: &mut FileServiceClient<Channel>,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err("File does not exist".into());
    }

    if !path.is_file() {
        return Err("Path is a directory, not a file".into());
    }
    let mut file = File::open(path).await?;
    let filename = path
        .file_name()
        .unwrap()
        .to_str()
        .ok_or("Filename contains invalid UTF-8")?
        .to_string();
    let upload_id = uuid::Uuid::new_v4().to_string();

    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tokio::spawn(async move {
        let mut buffer = vec![0u8; 8192];
        let mut chunk_index: u64 = 0;

        loop {
            match file.read(&mut buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let chunk = UploadChunk {
                        upload_id: upload_id.clone(),
                        filename: filename.to_string(),
                        chunk_index,
                        data: buffer[..n].to_vec(),
                    };

                    if tx.send(chunk).await.is_err() {
                        break;
                    }
                    chunk_index += 1;
                }
                Err(_) => break,
            }
        }
    });

    // Send stream to server
    let stream = ReceiverStream::new(rx);
    let response = client.upload(tonic::Request::new(stream)).await?;
    let _result = response.into_inner();

    Ok(())
}
