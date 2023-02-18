use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{bitmex::message::BitmexMessage, ExchangeId},
    subscription::trade::PublicTrade,
};
use barter_integration::model::{Exchange, Instrument, Side};
use chrono::{DateTime, Utc};
use serde::{
    de::{Error, Unexpected},
    Deserialize, Serialize,
};

/// Terse type alias for an [`BybitTrade`](BybitTrade) real-time trades WebSocket message.
pub type BitmexTradePayload = BitmexMessage<BitmexTrade>;

/// ### Raw Payload Examples
/// See docs: <https://www.bitmex.com/app/wsAPI#Response-Format>
/// #### Trade payload
/// ```json
/// {
///     "table": "trade",
///     "action": "insert",
///     "data": [
///         {
///             "timestamp": "2023-02-18T09:27:59.701Z",
///             "symbol": "XBTUSD",
///             "side": "Sell",
///             "size": 200,
///             "price": 24564.5,
///             "tickDirection": "MinusTick",
///             "trdMatchID": "31e50cb7-e005-a44e-f354-86e88dff52eb",
///             "grossValue": 814184,
///             "homeNotional": 0.00814184,
///             "foreignNotional": 200,
///             "trdType": "Regular"
///         }
///     ]
/// }
///```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BitmexTrade {
    #[serde(deserialize_with = "de_str_datetime_as_datetime_utc")]
    pub timestamp: DateTime<Utc>,

    pub symbol: String,

    pub side: Side,
    #[serde(rename = "size")]
    pub amount: f64,
    pub price: f64,

    #[serde(rename = "trdMatchID")]
    pub id: String,

    #[serde(rename = "tickDirection", skip)]
    pub tick_direction: String,
    #[serde(rename = "grossValue", skip)]
    pub gross_value: i64,
    #[serde(rename = "homeNotional", skip)]
    pub home_notional: f64,
    #[serde(rename = "foreignNotional", skip)]
    pub foreign_notional: i64,
    #[serde(rename = "trdType", skip)]
    pub trd_type: String,
}

/// Deserialize a [`BybitMessage`] "s" (eg/ "publicTrade.BTCUSDT") as the associated
/// [`SubscriptionId`].
///
/// eg/ "2023-02-18T09:27:59.701Z" to DateTime<Utc>
pub fn de_str_datetime_as_datetime_utc<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let input = <&str as serde::Deserialize>::deserialize(deserializer)?;
    let custom = DateTime::parse_from_rfc3339("2023-02-18T09:27:59.701Z");
    match custom {
        Ok(t) => {
            let utc: DateTime<Utc> = t.into();
            Ok(utc)
        }
        _ => Err(Error::invalid_value(
            Unexpected::Str(input),
            &"invalid message type expected pattern: <type>.<symbol>",
        )),
    }
}

impl From<(ExchangeId, Instrument, BitmexTradePayload)> for MarketIter<PublicTrade> {
    fn from(
        (exchange_id, instrument, trades): (ExchangeId, Instrument, BitmexTradePayload),
    ) -> Self {
        Self(
            trades
                .data
                .into_iter()
                .map(|trade| {
                    Ok(MarketEvent {
                        exchange_time: trade.timestamp,
                        received_time: Utc::now(),
                        exchange: Exchange::from(exchange_id),
                        instrument: instrument.clone(),
                        kind: PublicTrade {
                            id: trade.id,
                            price: trade.price,
                            amount: trade.amount,
                            side: trade.side,
                        },
                    })
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::error::SocketError;
        use chrono::{Duration, TimeZone};

        #[test]
        fn test_bitmex_trade() {
            struct TestCase {
                input: &'static str,
                expected: Result<BitmexTrade, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitTrade is deserialised
                TestCase {
                    input: r#"
                    {
                        "timestamp": "2023-02-18T09:27:59.701Z",
                        "symbol": "XBTUSD",
                        "side": "Sell",
                        "size": 200,
                        "price": 24564.5,
                        "tickDirection": "MinusTick",
                        "trdMatchID": "31e50cb7-e005-a44e-f354-86e88dff52eb",
                        "grossValue": 814184,
                        "homeNotional": 0.00814184,
                        "foreignNotional": 200,
                        "trdType": "Regular"
                    }
                    "#,
                    expected: Ok(BitmexTrade {
                        timestamp: Utc.with_ymd_and_hms(2023, 2, 18, 9, 27, 59).unwrap()
                            + Duration::milliseconds(701),
                        symbol: "XBTUSD".to_string(),
                        side: Side::Sell,
                        amount: 200.0,
                        price: 24564.5,
                        id: "31e50cb7-e005-a44e-f354-86e88dff52eb".to_string(),
                        // ignored
                        tick_direction: "".to_string(),
                        gross_value: 0,
                        home_notional: 0.0,
                        foreign_notional: 0,
                        trd_type: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BitmexTrade>(test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }

        #[test]
        fn test_bitmex_payload_trade() {
            struct TestCase {
                input: &'static str,
                expected: Result<BitmexTradePayload, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitTrade is deserialised
                TestCase {
                    input: r#"
                    {
                        "table": "trade",
                        "action": "insert",
                        "data": [
                            {
                                "timestamp": "2023-02-18T09:27:59.701Z",
                                "symbol": "XBTUSD",
                                "side": "Sell",
                                "size": 200,
                                "price": 24564.5,
                                "tickDirection": "MinusTick",
                                "trdMatchID": "31e50cb7-e005-a44e-f354-86e88dff52eb",
                                "grossValue": 814184,
                                "homeNotional": 0.00814184,
                                "foreignNotional": 200,
                                "trdType": "Regular"
                            }
                        ]
                    }
                    "#,
                    expected: Ok(BitmexTradePayload {
                        table: "trade".to_string(),
                        action: "insert".to_string(),
                        data: vec![BitmexTrade {
                            timestamp: Utc.with_ymd_and_hms(2023, 2, 18, 9, 27, 59).unwrap()
                                + Duration::milliseconds(701),
                            symbol: "XBTUSD".to_string(),
                            side: Side::Sell,
                            amount: 200.0,
                            price: 24564.5,
                            id: "31e50cb7-e005-a44e-f354-86e88dff52eb".to_string(),
                            // ignored
                            tick_direction: "".to_string(),
                            gross_value: 0,
                            home_notional: 0.0,
                            foreign_notional: 0,
                            trd_type: "".to_string(),
                        }],
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BitmexTradePayload>(test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }
    }
}