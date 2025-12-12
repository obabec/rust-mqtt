use tokio_test::assert_ok;

use crate::{packet::TxPacket, test::write::SliceWriter};

macro_rules! encode {
    ($p:expr, $e:expr) => {
        crate::test::tx::assert_encoded($p, $e).await
    };
}

pub async fn assert_encoded<const N: usize>(packet: impl TxPacket, encoded: [u8; N]) {
    let mut buffer = [0; N];
    let written = {
        let mut writer = SliceWriter::new(&mut buffer);
        assert_ok!(packet.send(&mut writer).await);
        writer.written()
    };
    assert_eq!(N, written);

    let written = &buffer[..written];

    assert_eq!(encoded, written);
}

pub(crate) use encode;
