use model::*;
use errors::*;
use url::Url;
use serde_json::from_str;

use tungstenite::connect;
use tungstenite::protocol::WebSocket;
use tungstenite::client::AutoStream;
use tungstenite::handshake::client::Response;

static WEBSOCKET_URL: &'static str = "wss://stream.binance.com:9443/ws/";

static WEBSOCKET_MULTI_STREAM: &'static str = "wss://stream.binance.com:9443/stream?streams="; // <streamName1>/<streamName2>/<streamName3>
// {"stream":"<streamName>","data":<rawPayload>}
static STREAM: &'static str = "stream";
static _DATA: &'static str = "data";

static OUTBOUND_ACCOUNT_INFO: &'static str = "outboundAccountInfo";
static EXECUTION_REPORT: &'static str = "executionReport";

static KLINE: &'static str = "kline";
static AGGREGATED_TRADE: &'static str = "aggTrade";
static DEPTH_ORDERBOOK : &'static str = "depthUpdate";
static PARTIAL_ORDERBOOK : &'static str = "lastUpdateId";

static DAYTICKER: &'static str = "24hrTicker";

pub trait UserStreamEventHandler {
    fn account_update_handler(&self, event: &AccountUpdateEvent);
    fn order_trade_handler(&self, event: &OrderTradeEvent);
}

pub trait MarketEventHandler {
    fn aggregated_trades_handler(&self, event: &TradesEvent);
    fn depth_orderbook_handler(&self, event: &DepthOrderBookEvent);
    fn partial_orderbook_handler(&self, order_book: &OrderBook);
}

pub trait DayTickerEventHandler {
    fn day_ticker_handler(&self, event: &[DayTickerEvent]);
}

pub trait KlineEventHandler {
    fn kline_handler(&self, event: &KlineEvent);
}

#[derive(Default)]
pub struct WebSockets {
    socket: Option<(WebSocket<AutoStream>, Response)>,
    user_stream_handler: Option<Box<UserStreamEventHandler>>,
    market_handler: Option<Box<MarketEventHandler>>,
    ticker_handler: Option<Box<DayTickerEventHandler>>,
    kline_handler: Option<Box<KlineEventHandler>>,
}

impl WebSockets {
    pub fn new() -> WebSockets {
        WebSockets {
            socket: None,
            user_stream_handler: None,
            market_handler: None,
            ticker_handler: None,
            kline_handler: None,
        }
    }

    pub fn connect(&mut self, endpoint: &str) -> Result<()> {
        let wss: String = format!("{}{}", WEBSOCKET_URL, endpoint);
        let url = Url::parse(&wss)?;

        match connect(url) {
            Ok(answer) => {
                self.socket = Some(answer);
                Ok(())
            }
            Err(e) => {
                bail!(format!("Error during handshake {}", e));
            }
        }
    }

    pub fn connect_multiple_streams(&mut self, endpoints: &Vec<String>) -> Result<()> {
        let wss: String = format!("{}{}", WEBSOCKET_MULTI_STREAM, endpoints.join("/"));
        let url = Url::parse(&wss)?;

        match connect(url) {
            Ok(answer) => {
                self.socket = Some(answer);
                Ok(())
            }
            Err(e) => {
                bail!(format!("Error during handshake {}", e));
            }
        }
    }

    pub fn add_user_stream_handler<H>(&mut self, handler: H)
    where
        H: UserStreamEventHandler + 'static,
    {
        self.user_stream_handler = Some(Box::new(handler));
    }

    pub fn add_market_handler<H>(&mut self, handler: H)
    where
        H: MarketEventHandler + 'static,
    {
        self.market_handler = Some(Box::new(handler));
    }

    pub fn add_day_ticker_handler<H>(&mut self, handler: H)
    where
        H: DayTickerEventHandler + 'static,
    {
        self.ticker_handler = Some(Box::new(handler));
    }

    pub fn add_kline_handler<H>(&mut self, handler: H)
    where
        H: KlineEventHandler + 'static,
    {
        self.kline_handler = Some(Box::new(handler));
    }

    fn handle_msg(&self, msg: &String) {
        if msg.find(OUTBOUND_ACCOUNT_INFO) != None {
            let account_update: AccountUpdateEvent = from_str(msg.as_str()).unwrap();

            if let Some(ref h) = self.user_stream_handler {
                h.account_update_handler(&account_update);
            }
        } else if msg.find(EXECUTION_REPORT) != None {
            let order_trade: OrderTradeEvent = from_str(msg.as_str()).unwrap();

            if let Some(ref h) = self.user_stream_handler {
                h.order_trade_handler(&order_trade);
            }
        } else if msg.find(AGGREGATED_TRADE) != None {
            let trades: TradesEvent = from_str(msg.as_str()).unwrap();

            if let Some(ref h) = self.market_handler {
                h.aggregated_trades_handler(&trades);
            }
        } else if msg.find(DAYTICKER) != None {
            let trades: Vec<DayTickerEvent> = from_str(msg.as_str()).unwrap();

            if let Some(ref h) = self.ticker_handler {
                h.day_ticker_handler(&trades);
            }
        } else if msg.find(KLINE) != None {
            let kline: KlineEvent = from_str(msg.as_str()).unwrap();

            if let Some(ref h) = self.kline_handler {
                h.kline_handler(&kline);
            }
        } else if msg.find(PARTIAL_ORDERBOOK) != None {
            let partial_orderbook: OrderBook = from_str(msg.as_str()).unwrap();

            if let Some(ref h) = self.market_handler {
                h.partial_orderbook_handler(&partial_orderbook);
            }
        } else if msg.find(DEPTH_ORDERBOOK) != None {
            let depth_orderbook: DepthOrderBookEvent = from_str(msg.as_str()).unwrap();

            if let Some(ref h) = self.market_handler {
                h.depth_orderbook_handler(&depth_orderbook);
            }
        } else if msg.find(STREAM) != None {
            let i_msg = msg.find("\"data\":");
            let i_end = msg.rfind("}");
            if let (Some(i_msg_), Some(i_end_)) = (i_msg, i_end) {
                let sub_string = msg.chars().skip(i_msg_).take(i_end_ - i_msg_ - 1).collect();
                self.handle_msg(&sub_string);
            };
        }
    }


    pub fn event_loop(&mut self) {
        loop {
            let msg_opt =
                match self.socket {
                    Some (ref mut socket) => {
                        let msg: String = socket.0.read_message().unwrap().into_text().unwrap().to_string();
                        Some(msg)
                    },
                    None => None
                };
            if let Some(ref m) = msg_opt {
                self.handle_msg(&m);
            }
           // if let Some(ref mut socket) = self.socket {
           //     let msg: String = socket.0.read_message().unwrap().into_text().unwrap();
//
           //     self.handle_msg(&msg);
           // }
        }
    }
}
