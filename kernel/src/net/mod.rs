use smoltcp::iface::*;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr};
use smoltcp::socket::{TcpSocket, TcpSocketBuffer, SocketSet, SocketHandle};
use smoltcp::time::Instant;
use smoltcp::iface::EthernetInterface;
use x86_64::instructions::interrupts::enable_interrupts_and_hlt;

mod dma;
mod e1000;
pub use dma::Dma;
pub use e1000::setup_1000e;

pub struct Network<'a, 'b, 'c> {
    iface: EthernetInterface<'a, 'b, 'c, e1000::E1000Net>,
    sockets: SocketSet<'a, 'b, 'c>,
    tcp_handle: SocketHandle,
}

impl<'a, 'b, 'c> Network<'a, 'b, 'c> {
    pub fn poll(&mut self) -> Result<bool, smoltcp::Error> {
        let now = Instant::from_millis(crate::time::SysTime::now().millis());
        self.iface.poll(&mut self.sockets, now)
    }

    pub fn wait_for_connection(&mut self) -> Result<(), smoltcp::Error> {
        loop {
            self.poll()?;
            let mut socket = self.sockets.get::<TcpSocket>(self.tcp_handle);
            if !socket.is_open() {
                socket.listen(1234).unwrap();
            }
            if socket.may_send() {
                return Ok(());
            }
            enable_interrupts_and_hlt();
        }
    }

    pub fn recv_nonblocking(&mut self, buf: &mut [u8]) -> Result<usize, smoltcp::Error> {
        self.poll()?;
        let mut socket = self.sockets.get::<TcpSocket>(self.tcp_handle);
        if socket.can_recv() {
            socket.recv_slice(buf)
        } else {
            Ok(0)
        }
    }

    pub fn recv(&mut self, mut buf: &mut [u8]) -> Result<(), smoltcp::Error> {
        while !buf.is_empty() {
            let amount = self.recv_nonblocking(buf)?;
            buf = &mut buf[amount..];
            if !buf.is_empty() {
                enable_interrupts_and_hlt();
            }
        }
        Ok(())
    }

    pub fn send(&mut self, mut data: &[u8]) -> Result<(), smoltcp::Error> {
        let mut socket = self.sockets.get::<TcpSocket>(self.tcp_handle);
        while !data.is_empty() {
            let amount = socket.send_slice(data)?;
            data = &data[amount..];
        }
        Ok(())
    }
}

pub fn init(device: e1000::E1000Net) -> Network<'static, 'static, 'static> {

    let ethernet_addr = EthernetAddress(device.mac_address());
    let ip_addrs = [
        IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24),
        IpCidr::new(IpAddress::v6(0xfdaa, 0, 0, 0, 0, 0, 0, 1), 64),
        IpCidr::new(IpAddress::v6(0xfe80, 0, 0, 0, 0, 0, 0, 1), 64)
    ];


    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let iface = EthernetInterfaceBuilder::new(device)
            .ethernet_addr(ethernet_addr)
            .neighbor_cache(neighbor_cache)
            .ip_addrs(ip_addrs)
            .finalize();


    let mut rx_buffer = Vec::new();
    rx_buffer.resize(65535, 0);
    let mut tx_buffer = Vec::new();
    tx_buffer.resize(65535, 0);
    let tcp_rx_buffer = TcpSocketBuffer::new(rx_buffer);
    let tcp_tx_buffer = TcpSocketBuffer::new(tx_buffer);
    let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

    let mut sockets = SocketSet::new(Vec::new());
    let tcp_handle = sockets.add(tcp_socket);

    Network {
        iface,
        sockets,
        tcp_handle
    }
}