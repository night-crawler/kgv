pub fn init_cursive_backend() -> std::io::Result<Box<dyn cursive::backend::Backend>> {
    let backend = cursive::backends::termion::Backend::init()?;
    let buffered_backend = cursive_buffered_backend::BufferedBackend::new(backend);
    Ok(Box::new(buffered_backend))
}