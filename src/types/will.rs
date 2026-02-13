use crate::{
    client::options::WillOptions,
    eio::Write,
    io::{err::WriteError, write::Writable},
    types::{MqttBinary, TopicName, VarByteInt},
    v5::property::{
        ContentType, CorrelationData, MessageExpiryInterval, PayloadFormatIndicator, ResponseTopic,
        WillDelayInterval,
    },
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Will<'w> {
    pub will_topic: TopicName<'w>,

    // Will properties
    pub will_delay_interval: Option<WillDelayInterval>,
    pub payload_format_indicator: Option<PayloadFormatIndicator>,
    pub message_expiry_interval: Option<MessageExpiryInterval>,
    pub content_type: Option<ContentType<'w>>,
    pub response_topic: Option<ResponseTopic<'w>>,
    pub correlation_data: Option<CorrelationData<'w>>,

    pub will_payload: MqttBinary<'w>,
}

impl<'w> From<WillOptions<'w>> for Will<'w> {
    fn from(options: WillOptions<'w>) -> Self {
        Self {
            will_topic: options.will_topic,
            will_delay_interval: match options.will_delay_interval {
                0 => None,
                i => Some(WillDelayInterval(i)),
            },
            payload_format_indicator: match options.is_payload_utf8 {
                false => None,
                true => Some(PayloadFormatIndicator(true)),
            },
            message_expiry_interval: options.message_expiry_interval.map(Into::into),
            content_type: options.content_type.map(Into::into),
            response_topic: options.response_topic.map(Into::into),
            correlation_data: options.correlation_data.map(Into::into),
            will_payload: options.will_payload,
        }
    }
}

impl<'p> Writable for Will<'p> {
    fn written_len(&self) -> usize {
        let will_properties_length = self.will_properties_length();

        will_properties_length.written_len()
            + will_properties_length.size()
            + self.will_topic.written_len()
            + self.will_payload.written_len()
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        let will_properties_length = self.will_properties_length();

        will_properties_length.write(write).await?;

        self.will_delay_interval.write(write).await?;
        self.payload_format_indicator.write(write).await?;
        self.message_expiry_interval.write(write).await?;
        self.content_type.write(write).await?;
        self.response_topic.write(write).await?;
        self.correlation_data.write(write).await?;

        self.will_topic.write(write).await?;
        self.will_payload.write(write).await?;

        Ok(())
    }
}

impl<'p> Will<'p> {
    pub fn will_properties_length(&self) -> VarByteInt {
        let will_properties_length = self.will_delay_interval.written_len()
            + self.payload_format_indicator.written_len()
            + self.message_expiry_interval.written_len()
            + self.content_type.written_len()
            + self.response_topic.written_len()
            + self.correlation_data.written_len();

        // Invariant: 196626 < VarByteInt::MAX_ENCODABLE
        // will delay interval: 5
        // payload format indicator: 2
        // message expiry interval: 5
        // content type: 65538
        // response topic: 65538
        // correlation data: 65538
        VarByteInt::new_unchecked(will_properties_length as u32)
    }
}
