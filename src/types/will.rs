use heapless::Vec;

use crate::{
    client::options::WillOptions,
    eio::Write,
    io::{err::WriteError, write::Writable},
    types::{MqttBinary, MqttStringPair, TopicName, VarByteInt},
    v5::property::{
        ContentType, CorrelationData, MessageExpiryInterval, PayloadFormatIndicator, ResponseTopic,
        UserProperty, WillDelayInterval,
    },
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Will<'w, const MAX_USER_PROPERTIES: usize> {
    pub will_topic: TopicName<'w>,

    // Will properties
    pub will_delay_interval: Option<WillDelayInterval>,
    pub payload_format_indicator: Option<PayloadFormatIndicator>,
    pub message_expiry_interval: Option<MessageExpiryInterval>,
    pub content_type: Option<ContentType<'w>>,
    pub response_topic: Option<ResponseTopic<'w>>,
    pub correlation_data: Option<CorrelationData<'w>>,
    pub user_properties: Vec<UserProperty<'w>, MAX_USER_PROPERTIES>,

    pub will_message: MqttBinary<'w>,
}

impl<'w, const MAX_USER_PROPERTIES: usize> From<WillOptions<'w>> for Will<'w, MAX_USER_PROPERTIES> {
    fn from(options: WillOptions<'w>) -> Self {
        Self {
            will_topic: options.will_topic,
            will_delay_interval: match options.will_delay_interval {
                0 => None,
                i => Some(WillDelayInterval(i)),
            },
            payload_format_indicator: options.payload_format_indicator.map(Into::into),
            message_expiry_interval: options.message_expiry_interval.map(Into::into),
            content_type: options.content_type.map(Into::into),
            response_topic: options.response_topic.map(Into::into),
            correlation_data: options.correlation_data.map(Into::into),
            user_properties: options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
            will_message: options.will_message,
        }
    }
}

impl<const MAX_USER_PROPERTIES: usize> Writable for Will<'_, MAX_USER_PROPERTIES> {
    fn written_len(&self) -> usize {
        let will_properties_length = self.will_properties_length();

        // max length = MAX_USER_PROPERTIES * 131077 + 327704
        //
        // will property length: 4
        // will properties: MAX_USER_PROPERTIES * 131077 + 196626
        // will topic: 65537
        // will message: 65537
        will_properties_length.written_len()
            + will_properties_length.size()
            + self.will_topic.written_len()
            + self.will_message.written_len()
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

        for user_property in &self.user_properties {
            user_property.write(write).await?;
        }

        self.will_topic.write(write).await?;
        self.will_message.write(write).await?;

        Ok(())
    }
}

impl<const MAX_USER_PROPERTIES: usize> Will<'_, MAX_USER_PROPERTIES> {
    pub fn will_properties_length(&self) -> VarByteInt {
        let will_properties_length = self.will_delay_interval.written_len()
            + self.payload_format_indicator.written_len()
            + self.message_expiry_interval.written_len()
            + self.content_type.written_len()
            + self.response_topic.written_len()
            + self.correlation_data.written_len()
            + self
                .user_properties
                .iter()
                .map(Writable::written_len)
                .sum::<usize>();

        // max length = MAX_USER_PROPERTIES * 131077 + 196626
        // Invariant: MAX_USER_PROPERTIES <= 2046 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // will delay interval: 5
        // payload format indicator: 2
        // message expiry interval: 5
        // content type: 65538
        // response topic: 65538
        // correlation data: 65538
        // user properties: MAX_USER_PROPERTIES * 131077
        VarByteInt::new_unchecked(will_properties_length as u32)
    }
}
