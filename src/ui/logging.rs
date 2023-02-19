use cursive::Cursive;

use crate::util::panics::OptionExt;

pub fn setup_logging(siv: &Cursive) {
    let home = home::home_dir().unwrap_or_log().join(".kgv").join("logs");
    flexi_logger::Logger::try_with_env_or_str("info")
        .expect("Could not create Logger from environment :(")
        .log_to_file_and_writer(
            flexi_logger::FileSpec::default()
                .directory(home)
                .suppress_timestamp(),
            cursive_flexi_logger_view::cursive_flexi_logger(siv),
        )
        .format(flexi_logger::colored_with_thread)
        .start()
        .expect("failed to initialize logger!");
}
