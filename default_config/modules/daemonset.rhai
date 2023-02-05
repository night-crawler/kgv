import "commons" as cm;

fn state(resource) {
    if cm::is_deleted(resource) {
        "Deleted"
    } else {
        "Active"
    }
}
