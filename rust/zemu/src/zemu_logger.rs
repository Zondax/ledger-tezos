use arrayvec::ArrayString;
use log::{Level, Metadata, Record};

#[derive(Default)]
pub struct ZemuLog;

impl log::Log for ZemuLog {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        crate::zemu_log(&Self::read_record(record))
    }

    fn flush(&self) {}
}

impl ZemuLog {
    fn read_record(record: &Record) -> ArrayString<256> {
        let mut s = ArrayString::<256>::new();

        match core::fmt::write(
            &mut s,
            format_args!(
                "[{}] {} @ {}\n\x00",
                record.level().as_str(),
                record.target(),
                record.args()
            ),
        ) {
            Ok(_) => s,
            //this is always valid utf8
            Err(_) => ArrayString::from("[ERROR] Bad formatting").unwrap(),
        }
    }

    ///Install this logger as the global logger
    pub fn install() -> Result<(), log::SetLoggerError> {
        unsafe { log::set_logger_racy(&Self {}) }?;
        log::set_max_level(Level::Trace.to_level_filter());
        Ok(())
    }
}
