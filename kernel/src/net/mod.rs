use smoltcp::iface::*;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr};
use smoltcp::socket::{TcpSocket, TcpSocketBuffer, SocketSet};
use smoltcp::time::Instant;

mod dma;
mod e1000;
pub use dma::Dma;
pub use e1000::setup_1000e;


pub fn init(device: e1000::E1000Net) {

    let ethernet_addr = EthernetAddress(device.mac_address());
    let ip_addrs = [
        IpCidr::new(IpAddress::v4(10, 0, 2, 1), 28),
        IpCidr::new(IpAddress::v6(0xfdaa, 0, 0, 0, 0, 0, 0, 1), 64),
        IpCidr::new(IpAddress::v6(0xfe80, 0, 0, 0, 0, 0, 0, 1), 64)
    ];


    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let mut iface = EthernetInterfaceBuilder::new(device)
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

    loop {
        let now = Instant::from_millis(crate::time::SysTime::now().millis());
        let res = iface.poll(&mut sockets, now);
        if res.is_ok() {
            printk!("{:?}\n", res);
        }
        for _ in 0..20 {
            unsafe { asm!("hlt") };
        }
    }
}