/// Developer error message macro, for when a developer has made a mistake in the code.
#[macro_export]
macro_rules! dev_err {
        ($what: expr) => {
                format!(
                        "Developer error, please report this error: {} @{}:{}",
                        $what,
                        file!(),
                        line!()
                )
        };
}
#[macro_export]
macro_rules! unexpected_err {
        ($what: expr) => {
                format!(
                        "Unexpected error, please report this error: {} @{}:{}",
                        $what,
                        file!(),
                        line!()
                )
        };
}
