pub mod config;
pub mod fileservice {
    tonic::include_proto!("fileservice");
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("fileservice_descriptor");
}
pub mod tui;
