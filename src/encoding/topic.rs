/*
 * MIT License
 *
 * Copyright (c) [2022] [Ondrej Babec <ond.babec@gmail.com>]
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use crate::{encoding::EncodedString, interface::{RetainHandling, Topic}};

/// Topic filter serves as bound for topic selection and subscription options for `SUBSCRIBE` packet
#[derive(Debug, Default)]
pub struct TopicFilter<'a> {
    pub filter: EncodedString<'a>,
    pub sub_options: u8,
}

impl TopicFilter<'_> {
    pub fn new() -> Self {
        Self {
            filter: EncodedString::new(),
            sub_options: 0,
        }
    }

    pub fn encoded_len(&self) -> u16 {
        self.filter.len + 3
    }
}


impl<'a> From<Topic<'a>> for TopicFilter<'a> {
    fn from(value: Topic<'a>) -> Self {
        let retain_handling_bits = match value.retain_handling {
            RetainHandling::AlwaysSend => 0x00,
            RetainHandling::SendIfNotSubscribedBefore => 0x10,
            RetainHandling::NeverSend => 0x20,
        };

        let retain_as_published_bit = match value.retain_as_published {
            true => 0x08,
            false => 0x00,
        };

        let no_local_bit = match value.no_local {
            true => 0x04,
            false => 0x00,
        };

        let qos_bits = value.qos.into_subscribe_bits();

        let subscribe_options_bits =
            retain_handling_bits | retain_as_published_bit | no_local_bit | qos_bits;

        let mut filter = EncodedString::new();
        filter.string = value.topic_name;
        filter.len = value.topic_name.len() as u16;

        Self {
            filter,
            sub_options: subscribe_options_bits,
        }
    }
}