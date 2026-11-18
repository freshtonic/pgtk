//! Runs a builder-based proxy which logs every frontend and backend protocol message.

use std::{convert::Infallible, env, error::Error, io, net::SocketAddr, sync::Arc};

use pgtk_proto::{
    BackendMessage, BoundedPipeline, CancellationPolicy, Client, ClientConnectionContext,
    ClientTlsPolicy, ConnectTarget, ForwardedMessage, FrontendMessage, InitialServerContext,
    Intermediary, IntermediaryMiddleware, Server, ServerConnectionContext, ServerIdentity,
    ServerIdentityProvider, ServerTlsPolicy, StartupParameters, StartupRouteResolver,
    TrustClientAuthentication, TrustIdentity, TrustServerAuthentication,
};
use pgtk_test_server::Server as TestServer;
use rcgen::generate_simple_self_signed;
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use tokio::net::{TcpListener, TcpStream};

struct ProtocolState {
    connection: u64,
}

#[derive(Clone, Copy)]
struct ProtocolLogger;

impl
    IntermediaryMiddleware<
        ProtocolState,
        ServerConnectionContext<SocketAddr, TrustIdentity>,
        ClientConnectionContext<()>,
    > for ProtocolLogger
{
    type Error = std::convert::Infallible;

    async fn frontend(
        &mut self,
        _: &ServerConnectionContext<SocketAddr, TrustIdentity>,
        _: &ClientConnectionContext<()>,
        state: &mut ProtocolState,
        message: FrontendMessage,
    ) -> Result<pgtk_proto::FrontendMiddlewareOutput, Self::Error> {
        println!("[{}] client -> server: {message:?}", state.connection);
        Ok(pgtk_proto::FrontendMiddlewareOutput::Forward(message))
    }

    async fn backend(
        &mut self,
        _: &ServerConnectionContext<SocketAddr, TrustIdentity>,
        _: &ClientConnectionContext<()>,
        state: &mut ProtocolState,
        message: BackendMessage,
    ) -> Result<pgtk_proto::BackendMiddlewareOutput, Self::Error> {
        println!("[{}] server -> client: {message:?}", state.connection);
        Ok(pgtk_proto::BackendMiddlewareOutput::Forward(message))
    }
}

#[derive(Clone)]
struct ExampleTlsIdentity {
    identity: ServerIdentity,
}

impl ExampleTlsIdentity {
    fn generate() -> Result<Self, Box<dyn Error>> {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let generated =
            generate_simple_self_signed(vec!["localhost".to_owned(), "127.0.0.1".to_owned()])?;
        let certificate = CertificateDer::from(generated.cert.der().to_vec());
        let key = PrivateKeyDer::try_from(generated.signing_key.serialize_der())?;
        let config = Arc::new(
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![certificate.clone()], key)?,
        );
        Ok(Self {
            identity: ServerIdentity::new(config, certificate),
        })
    }
}

impl ServerIdentityProvider for ExampleTlsIdentity {
    type Error = Infallible;

    fn resolve(&self) -> Result<ServerIdentity, Self::Error> {
        Ok(self.identity.clone())
    }
}

struct ExampleUpstream {
    address: SocketAddr,
    _server: Option<TestServer>,
}

impl ExampleUpstream {
    async fn resolve(configured: Option<&str>) -> Result<Self, Box<dyn Error>> {
        if let Some(configured) = configured {
            let address: SocketAddr = configured.parse()?;
            TcpStream::connect(address).await.map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "cannot connect to PostgreSQL at {address}: {error}; start that server or \
                         omit the upstream argument to use the example server"
                    ),
                )
            })?;
            return Ok(Self {
                address,
                _server: None,
            });
        }
        let version = env::var("PGTK_PROTO_POSTGRES_VERSION").unwrap_or_else(|_| "18".to_owned());
        if !matches!(version.as_str(), "14" | "15" | "16" | "17" | "18") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PGTK_PROTO_POSTGRES_VERSION must be 14, 15, 16, 17, or 18",
            )
            .into());
        }
        let server = TestServer::builder_of(&format!("pg{version}"))
            .tcp()
            .init_sql(include_str!("../sql_logging_proxy/customer_orders.sql"))
            .start();
        Ok(Self {
            address: server.tcp_address(),
            _server: Some(server),
        })
    }
}

#[derive(Clone, Copy)]
struct Route(SocketAddr);

impl StartupRouteResolver<SocketAddr> for Route {
    type Error = Infallible;

    async fn resolve(
        &self,
        _: StartupParameters,
        _: InitialServerContext<'_, SocketAddr>,
    ) -> Result<ConnectTarget, Self::Error> {
        Ok(ConnectTarget::new(self.0.to_string()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listen = address(1, "127.0.0.1:6432")?;
    let upstream = ExampleUpstream::resolve(env::args().nth(2).as_deref()).await?;
    let upstream_address = upstream.address;
    let tls = ExampleTlsIdentity::generate()?;
    let listener = TcpListener::bind(listen).await?;
    println!("protocol logging proxy listening on {listen}; upstream is {upstream_address}");
    serve(listener, upstream_address, tls).await?;
    Ok(())
}

async fn serve(
    listener: TcpListener,
    upstream: SocketAddr,
    tls: ExampleTlsIdentity,
) -> io::Result<()> {
    let mut next = 1_u64;
    let workers = Arc::new(tokio::sync::Semaphore::new(64));
    loop {
        let (transport, peer) = listener.accept().await?;
        let worker = Arc::clone(&workers)
            .acquire_owned()
            .await
            .map_err(|_| io::Error::other("connection worker pool closed"))?;
        let connection = next;
        next = next.wrapping_add(1);
        let tls = tls.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("connection runtime");
            runtime.block_on(async move {
                let _worker = worker;
                if let Err(error) =
                    Box::pin(proxy_connection(transport, peer, upstream, tls, connection)).await
                {
                    eprintln!("connection {connection}: {error}");
                }
            });
        });
    }
}

async fn proxy_connection(
    transport: TcpStream,
    peer: SocketAddr,
    upstream: SocketAddr,
    tls: ExampleTlsIdentity,
    connection: u64,
) -> io::Result<()> {
    let server = Server::builder()
        .tls(ServerTlsPolicy::Required(tls))
        .authentication(TrustServerAuthentication)
        .build()
        .map_err(other)?;
    let client = Client::builder()
        .connector(move |_| TcpStream::connect(upstream))
        .tls(ClientTlsPolicy::Disabled)
        .authentication(TrustClientAuthentication)
        .build()
        .map_err(other)?;
    let intermediary = Intermediary::builder()
        .server(server)
        .client(client)
        .startup_resolver(Route(upstream))
        .cancellation(CancellationPolicy::Reject)
        .pipeline(BoundedPipeline::new(64).expect("non-zero queued-request limit"))
        .middleware(
            |_: &ServerConnectionContext<SocketAddr, TrustIdentity>,
             _: &ClientConnectionContext<()>| ProtocolLogger,
        )
        .build()
        .map_err(other)?;
    let accepted = Box::pin(intermediary.accept(transport, peer, ProtocolState { connection }))
        .await
        .map_err(other)?;
    let mut session = accepted.into_session();
    loop {
        match session.forward_next().await {
            Ok(ForwardedMessage::Frontend(FrontendMessage::Terminate)) => {
                let _ = session.teardown();
                return Ok(());
            }
            Ok(_) => {}
            Err(error) => {
                let error = other(error);
                let _ = session.teardown();
                return Err(error);
            }
        }
    }
}

fn address(argument: usize, default: &str) -> Result<SocketAddr, Box<dyn Error>> {
    Ok(env::args()
        .nth(argument)
        .as_deref()
        .unwrap_or(default)
        .parse()?)
}

fn other(error: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("{error:?}"))
}
