macro_rules! assert_ok {
    ($e:expr) => {{
        use std::result::Result::*;
        match $e {
            Ok(v) => v,
            Err(e) => panic!("assertion failed: Err({:?})", e),
        }
    }};
    ($e:expr, $t:literal) => {{
        use std::result::Result::*;
        match $e {
            Ok(v) => v,
            Err(e) => crate::common::fmt::error_panic!($t, e),
        }
    }};
}

macro_rules! assert_subscribe {
    ($client:expr, $options:expr, $topic:expr) => {
        let qos = $options.qos;
        let granted_qos = assert_ok!(
            crate::common::utils::subscribe(&mut $client, $options, $topic).await,
            "Failed to subscribe"
        );
        assert_eq!(
            qos, granted_qos,
            "Subscribed with different quality of service than expected"
        );
    };
}

macro_rules! assert_unsubscribe {
    ($client:expr, $topic:expr) => {
        assert_ok!(
            crate::common::utils::unsubscribe(&mut $client, $topic).await,
            "Failed to unsubscribe"
        );
    };
}

macro_rules! assert_recv {
    ($client:expr) => {{ assert_ok!(crate::common::utils::receive_and_complete(&mut $client).await) }};
}

macro_rules! assert_recv_excl {
    ($client:expr, $topic:expr) => {{
        let p = assert_ok!(crate::common::utils::receive_and_complete(&mut $client).await);
        assert_eq!(
            &$topic, &p.topic,
            "expected topic (left) != received topic (right)"
        );

        p
    }};
}

macro_rules! assert_published {
    ($client:expr, $options:expr, $msg:expr) => {
        assert_ok!(crate::common::utils::publish_and_complete(&mut $client, &$options, $msg).await)
    };
}

pub(crate) use assert_ok;
pub(crate) use assert_published;
pub(crate) use assert_recv;
pub(crate) use assert_recv_excl;
pub(crate) use assert_subscribe;
pub(crate) use assert_unsubscribe;
