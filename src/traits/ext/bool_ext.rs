pub(crate) trait BoolExt {
    fn as_on_off(&self) -> &str;
    fn as_yes_no(&self) -> &str;
}

impl BoolExt for bool {
    fn as_on_off(&self) -> &str {
        if *self {
            "on"
        } else {
            "off"
        }
    }

    fn as_yes_no(&self) -> &str {
        if *self {
            "yes"
        } else {
            "no"
        }
    }
}
