/// Unexpected oopsies.
#[macro_export]
macro_rules! err_msg {
    ($what: expr) => {
        concat!(
            "Unexpected error, please report this on github : ",
            $what,
            " @",
            file!(),
            ":",
            line!()
        )
    };
}
