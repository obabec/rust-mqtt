macro_rules! warn_inspect {
    ($e:expr, $t:literal) => {{
        use std::result::Result::*;
        let r = $e;
        match r {
            Ok(_) => {}
            Err(ref e) => ::log::warn!("{}: {:?}", $t, e),
        }

        r
    }};
}

macro_rules! error_panic {
    ($t:literal, $e:expr) => {{
        ::log::error!("{}: {:?}", $t, $e);
        panic!("{}: {:?}", $t, $e);
    }};
}

pub(crate) use error_panic;
pub(crate) use warn_inspect;
