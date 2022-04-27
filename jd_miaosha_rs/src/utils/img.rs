use iced::*;

lazy_static!(
    pub static ref RIGHT_IMAGE_HANDLE:image::Handle = image::Handle::from_memory(include_bytes!("../../assets/images/right.png").to_vec());
    pub static ref WRONG_IMAGE_HANDLE:image::Handle = image::Handle::from_memory(include_bytes!("../../assets/images/wrong.png").to_vec());
    pub static ref LOADING_IMAGE_HANDLE:image::Handle = image::Handle::from_memory(include_bytes!("../../assets/images/loading.gif").to_vec());
);

pub fn create_image(img_handle:&image::Handle) -> image::Image{
    Image::new(img_handle.clone())
        .width(20.into())
        .height(20.into())
}


pub fn right_image() ->image::Image{
    create_image(&*RIGHT_IMAGE_HANDLE)
}

pub fn wrong_image() ->image::Image{
    create_image(&*WRONG_IMAGE_HANDLE)
}

pub fn loading_image() ->image::Image{
    create_image(&*LOADING_IMAGE_HANDLE)
}