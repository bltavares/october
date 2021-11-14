fn main() {
    self::cli::main();
}

mod adapter {
    use macaddr::MacAddr6;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::str::FromStr;

    pub fn deserialize<'de, D>(data: D) -> Result<MacAddr6, D::Error>
    where
        D: Deserializer<'de>,
    {
        let addr = String::deserialize(data)?;
        let mac = MacAddr6::from_str(&addr).map_err(de::Error::custom)?;
        Ok(mac)
    }

    pub fn serialize<S>(data: &MacAddr6, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{}", data).serialize(serializer)
    }
}

mod wol {
    use lazy_static::lazy_static;
    use std::net::SocketAddr;

    lazy_static! {
        pub static ref SOURCE: SocketAddr = SocketAddr::from(([0, 0, 0, 0], 0));
        pub static ref NETWORK: SocketAddr = SocketAddr::from(([255, 255, 255, 255], 9));
    }

    const MAC_SIZE: usize = 6;
    const MAC_PER_MAGIC: usize = 16;
    static HEADER: [u8; 6] = [0xFF; 6];

    fn extend_mac(mac: &[u8]) -> Vec<u8> {
        std::iter::repeat(mac)
            .take(MAC_PER_MAGIC)
            .flatten()
            .copied()
            .collect()
    }

    pub fn create_packet_bytes(mac: &[u8]) -> Vec<u8> {
        let mut packet = Vec::with_capacity(HEADER.len() + MAC_SIZE * MAC_PER_MAGIC);

        packet.extend(HEADER.iter());
        packet.extend(extend_mac(mac));

        packet
    }
}

mod awake {
    use axum::{
        extract::Form,
        response::{Html, IntoResponse},
    };
    use macaddr::MacAddr6;
    use serde::Deserialize;
    use tokio::net::UdpSocket;

    use crate::wol;

    #[derive(Debug, Deserialize)]
    pub struct Input {
        #[serde(deserialize_with = "crate::adapter::deserialize")]
        pub mac: MacAddr6,
    }

    pub async fn handler(Form(data): Form<Input>) -> impl IntoResponse {
        let packet = wol::create_packet_bytes(data.mac.as_bytes());

        let socket = UdpSocket::bind(*wol::SOURCE)
            .await
            .expect("Could not bind broadcast socket");
        socket
            .set_broadcast(true)
            .expect("Could not set broadcast bit");
        socket
            .send_to(&packet, *wol::NETWORK)
            .await
            .expect("Could not send broadcast packet");

        Html("Magic bytes sent.")
    }
}

mod index {
    use axum::{
        extract::Extension,
        response::{Html, IntoResponse},
    };
    use handlebars::Handlebars;
    use macaddr::MacAddr6;
    use serde::{Deserialize, Serialize};
    use std::collections::HashSet;

    /// Embedded index template
    pub const TEMPLATE: &str = include_str!("index.hbs");

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
    pub struct Address {
        pub name: String,
        #[serde(with = "crate::adapter")]
        pub mac: MacAddr6,
    }

    pub type State = std::sync::Arc<HashSet<Address>>;

    pub async fn handler(
        Extension(engine): Extension<Handlebars<'_>>,
        Extension(adressess): Extension<State>,
    ) -> impl IntoResponse {
        let content = engine
            .render("index", adressess.as_ref())
            .expect("Invalid handlebar syntax on index.hbs");
        Html(content)
    }
}

mod cli {
    use axum::{
        error_handling::HandleErrorExt,
        http::StatusCode,
        routing::service_method_routing as service,
        routing::{get, post},
        AddExtensionLayer, Router,
    };
    use csv::StringRecord;
    use handlebars::Handlebars;
    use std::{
        collections::HashSet, convert::Infallible, io::BufReader, net::SocketAddr, path::PathBuf,
        sync::Arc,
    };
    use structopt::StructOpt;
    use tower_http::{services::ServeDir, trace::TraceLayer};

    use crate::{awake, index};

    #[derive(Debug, StructOpt)]
    #[structopt(
        name = "October",
        about = "Wake-on-lan webapp - Wake me up, when september ends",
        rename_all = "kebab-case"
    )]
    struct Args {
        /// Monitors the index.hbs file for changes on every request. Disabled by default.
        #[structopt(short = "d", long, requires("template"))]
        autoreload: bool,

        /// The name of the Handlebars file template. When not present it will use the internal template.
        #[structopt(short = "t", long, env = "OCTOBER_TEMPLATE")]
        template: Option<PathBuf>,

        /// The address for the server to listen to
        #[structopt(short, long, default_value = "0.0.0.0:3493", env = "OCTOBER_ADDR")]
        listen: SocketAddr,

        /// A .csv file containing MAC addresses to wake up
        #[structopt(short = "a", long, env = "OCTOBER_ADDRESSES")]
        addresses: PathBuf,
    }

    fn read_addresses(file: PathBuf) -> std::io::Result<HashSet<index::Address>> {
        let file = std::fs::File::open(file)?;
        let reader = BufReader::new(file);
        let mut content = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(reader);
        let mut addresses = HashSet::new();

        let file_header = StringRecord::from(vec!["name", "mac"]);

        for (index, record) in content.records().enumerate() {
            let record = match record {
                Ok(record) => record,
                Err(err) => {
                    tracing::error!(error=?err, "Invalid record in address file");
                    continue;
                }
            };

            if let Ok(entry) = record.deserialize(Some(&file_header)) {
                addresses.insert(entry);
            } else {
                tracing::error!(index = index, "Invalid record in address file");
            }
        }

        Ok(addresses)
    }

    #[tokio::main]
    pub async fn main() {
        let args = Args::from_args();

        // Set the RUST_LOG, if it hasn't been explicitly defined
        if std::env::var_os("RUST_LOG").is_none() {
            std::env::set_var("RUST_LOG", "october=info,tower_http=info");
        }

        tracing_subscriber::fmt::init();

        tracing::info!(initial_options = ?args, "starting up");

        let file = args.template;
        let mut engine = Handlebars::new();
        engine.set_dev_mode(args.autoreload);

        if let Some(file) = file {
            engine
                .register_template_file("index", &file)
                .unwrap_or_else(|_| panic!("File '{:?}' not found on current dir", file));
        } else {
            engine
                .register_template_string("index", index::TEMPLATE)
                .expect("Mal-formatted internal template. Report on this issue");
        }

        let addresses = {
            match read_addresses(args.addresses) {
                Ok(content) => {
                    tracing::info!(addresses = ?content, "starting with registered addresses");
                    content
                }
                Err(e) => {
                    tracing::error!(error=?e, "Invalid address file. Initializing with empty addresses.");
                    HashSet::new()
                }
            }
        };

        let app = Router::new()
            .fallback(
                service::get(ServeDir::new(".")).handle_error(|error: std::io::Error| {
                    Ok::<_, Infallible>((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    ))
                }),
            )
            .route("/", get(index::handler))
            .route("/awake", post(awake::handler))
            .layer(AddExtensionLayer::new(engine))
            .layer(AddExtensionLayer::new(Arc::new(addresses)))
            .layer(TraceLayer::new_for_http());

        let address = args.listen;
        tracing::info!("Listening on {}", &address);
        hyper::Server::bind(&address)
            .serve(app.into_make_service())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Could not start server on the current address: {:?}",
                    address
                );
            });
    }
}
