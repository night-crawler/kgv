use cursive::Cursive;

use crate::util::paths::LOGS_DIR;

pub fn setup_logging(siv: &Cursive) {
    flexi_logger::Logger::try_with_env_or_str("info")
        .expect("Could not create Logger from environment :(")
        .log_to_file_and_writer(
            flexi_logger::FileSpec::default()
                .directory(LOGS_DIR.clone())
                .suppress_timestamp(),
            cursive_flexi_logger_view::cursive_flexi_logger(siv),
        )
        .format(flexi_logger::colored_with_thread)
        .start()
        .expect("failed to initialize logger!");
}
