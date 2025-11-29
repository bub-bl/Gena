#[derive(Debug)]
pub enum ResourceLoadingError {
    ImageLoadingError(image::ImageError),
    TextureCreationError(String),
    IoError(std::io::Error),
}

impl From<image::ImageError> for ResourceLoadingError {
    fn from(err: image::ImageError) -> Self {
        ResourceLoadingError::ImageLoadingError(err)
    }
}

impl From<std::io::Error> for ResourceLoadingError {
    fn from(err: std::io::Error) -> Self {
        ResourceLoadingError::IoError(err)
    }
}

pub trait GameResource {
    fn load_from_file(path: &str) -> Result<Self, ResourceLoadingError>
    where
        Self: Sized;

    fn save_to_file(&self) -> Result<(), ResourceLoadingError>;
    fn data(&self) -> &[u8];
}
