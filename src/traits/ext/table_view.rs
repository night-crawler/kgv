pub trait TableViewExt<T> {
    fn add_or_update_resource(&mut self, resource: T);
}
