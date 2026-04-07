//! NetworkTopology — Pipeline #123
//!
//! Logical and physical network topology analysis: host discovery, service
//! mapping, flow analysis, subnet structure, routing, BGP/routing protocols,
//! inter-service communication graphs, SDN overlays, and network-level
//! vulnerability/resilience mapping.
//!
//! DISTINCT FROM:
//!   - Electromagnetic (118): RF physical layer. Network Topology is the
//!     LOGICAL layer (L2/L3/L4+) — what connects to what, how packets flow.
//!     EM tells you what frequencies are occupied; Network tells you what
//!     services are running and how they communicate.
//!   - Code (101): application-level code. Network is infrastructure-level
//!     connectivity regardless of what code runs on hosts.
//!
//! CROSS-LINKS:
//!   101 (Code)    → services map to code implementations
//!   109 (3D)      → physical network placed in 3D building/campus space
//!   117 (Geo)     → network nodes geo-referenced (ISP/internet topology)
//!   118 (EM)      → RF as physical layer of wireless network
//!   122 (Control) → network control plane feeds control systems
//!   124 (Radar)   → radar data transport networks
//!
//! STORAGE: ZSEI containers under /Modalities/NetworkTopology/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ─────────────────────────────────────────────────────────────────────────────
// INPUT TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum NetworkModalityAction {
    /// Analyze a network topology from captured/configured data
    Analyze {
        data: NetworkDataSource,
        discover_hosts: bool,
        map_services: bool,
        analyze_flows: bool,
        detect_anomalies: bool,
        compute_resilience: bool,
    },
    /// Create graph from analysis result
    CreateGraph {
        analysis: NetworkAnalysisResult,
        project_id: u64,
    },
    /// Update graph with new scan/flow data
    UpdateGraph {
        graph_id: u64,
        updates: Vec<NetworkUpdate>,
        project_id: u64,
    },
    /// Discover hosts and services on a network
    DiscoverNetwork {
        target: NetworkTarget,
        scan_type: ScanType,
        port_range: Option<PortRange>,
        timeout_ms: u32,
    },
    /// Analyze packet capture for flow-level view
    AnalyzeCapture {
        pcap_path: String,
        flow_timeout_sec: u32,
        layer_depth: u8,            // 2=Ethernet, 3=IP, 4=TCP/UDP, 7=App
    },
    /// Map BGP/routing topology from routing tables / looking glass
    AnalyzeRoutingTopology {
        routing_data: RoutingDataSource,
        include_as_paths: bool,
    },
    /// Compute network resilience metrics
    ComputeResilience {
        graph_id: u64,
        metrics: Vec<ResilienceMetric>,
    },
    /// Find shortest/critical paths between nodes
    PathAnalysis {
        graph_id: u64,
        source_node_id: u64,
        target_node_id: u64,
        path_type: PathType,
    },
    /// Detect topology anomalies (new hosts, unexpected flows, changes)
    DetectAnomalies {
        graph_id: u64,
        baseline_graph_id: Option<u64>,
        anomaly_types: Vec<AnomalyType>,
    },
    /// Segment network into logical zones
    SegmentNetwork {
        graph_id: u64,
        method: SegmentationMethod,
    },
    QueryGraph { graph_id: u64, query: NetworkGraphQuery },
    GetGraph { graph_id: u64 },
    TriggerSemanticHook { graph_id: u64, hook: NetworkSemanticHook },
    ExportProduct { graph_id: u64, format: NetworkExportFormat },
    StreamToUI { graph_id: u64, session_id: String, display_mode: NetworkDisplayMode },
    HeadlessProcess { graph_id: u64, operations: Vec<NetworkOperation> },
}

// ─────────────────────────────────────────────────────────────────────────────
// DATA SOURCES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkDataSource {
    /// nmap XML output
    NmapXML { file_path: String },
    /// PCAP / network capture
    PcapFile { file_path: String, link_type: LinkType },
    /// NetFlow / IPFIX export
    NetFlowFile { file_path: String, format: FlowFormat },
    /// SNMP walk results
    SNMPWalk { file_path: String, community: Option<String> },
    /// BGP routing table dump (MRT format or text)
    BGPDump { file_path: String, format: BGPFormat },
    /// Cloud provider network config (AWS VPC, Azure VNet, GCP)
    CloudTopology { provider: CloudProvider, config_path: String },
    /// Kubernetes network config / service mesh
    KubernetesTopology { kubeconfig_path: Option<String>, namespace: Option<String> },
    /// LLDP / CDP neighbor table
    LLDPNeighbors { file_path: String },
    /// Zeek/Bro log directory
    ZeekLogs { log_dir: String },
    /// Firewall rule set
    FirewallRules { file_path: String, format: FirewallFormat },
    /// SDN controller topology (ONOS, OpenDaylight, etc.)
    SDNController { endpoint: String, controller_type: SDNControllerType },
    /// IPv6 neighbor discovery
    IPv6Neighbors { file_path: String },
    /// Live capture / streaming from network tap
    LiveCapture { interface: String, filter: Option<String> },
    /// Multi-source composite
    MultiSource { sources: Vec<NetworkDataSource>, merge_strategy: MergeStrategy },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkType { Ethernet, WiFi, Loopback, PPP, VLAN, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowFormat { NetFlow_v5, NetFlow_v9, IPFIX, sFlow, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BGPFormat { MRT, TextDump, GoBGP_JSON, OpenBGPD, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider { AWS, Azure, GCP, OracleCloud, Cloudflare, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FirewallFormat { iptables, nftables, PF, Cisco_ACL, FortiGate, PaloAlto, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SDNControllerType { ONOS, OpenDaylight, Ryu, Floodlight, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy { Union, Intersection, PriorityFirst, TimestampLatest }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanType { Ping, SYN, UDP, Full_Connect, Version_Detection, OS_Detection, Script_Scan, Stealth }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRange { pub start: u16, pub end: u16 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathType { Shortest, AllPaths, CriticalPath, RedundantPaths }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResilienceMetric { NodeConnectivity, EdgeConnectivity, Diameter, AveragePathLength, ClusteringCoefficient, BetweennessCentrality, VulnerableNodes }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType { NewHost, NewService, UnusualFlow, PortScan, BandwidthAnomaly, RoutingChange, TopologyChange }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentationMethod { VLAN, Subnet, ServiceGroup, SecurityZone, Community_Detection, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTarget {
    pub cidr: Option<String>,           // e.g., "192.168.1.0/24"
    pub host_list: Vec<String>,
    pub exclude_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingDataSource {
    BGP_Table { file_path: String, format: BGPFormat },
    OSPF_LSDB { file_path: String },
    ISIS_LSDB { file_path: String },
    StaticRoutes { file_path: String },
    LookingGlass { endpoint: String },
}

// ─────────────────────────────────────────────────────────────────────────────
// ANALYSIS RESULT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkAnalysisResult {
    pub analysis_id: u64,
    pub source_description: String,
    pub capture_timestamp: f64,

    // HOST INVENTORY
    pub hosts: Vec<NetworkHost>,
    pub subnets: Vec<Subnet>,

    // NETWORK DEVICES
    pub routers: Vec<Router>,
    pub switches: Vec<Switch>,
    pub firewalls: Vec<FirewallDevice>,
    pub load_balancers: Vec<LoadBalancer>,
    pub wireless_aps: Vec<WirelessAP>,

    // SERVICES
    pub services: Vec<NetworkService>,
    pub service_dependencies: Vec<ServiceDependency>,

    // FLOWS
    pub flow_paths: Vec<FlowPath>,
    pub top_talkers: Vec<TopTalker>,
    pub protocol_distribution: Vec<ProtocolStat>,

    // ROUTING
    pub routing_entries: Vec<RoutingEntry>,
    pub as_paths: Vec<ASPath>,
    pub bgp_peers: Vec<BGPPeer>,

    // SEGMENTATION
    pub vlans: Vec<VLAN>,
    pub security_zones: Vec<SecurityZone>,
    pub network_segments: Vec<NetworkSegment>,

    // TOPOLOGY METRICS
    pub total_host_count: u32,
    pub total_service_count: u32,
    pub total_flow_count: u32,
    pub topology_type: TopologyType,
    pub resilience_metrics: Option<ResilienceMetrics>,

    // ANOMALIES
    pub anomalies: Vec<NetworkAnomaly>,

    // CLOUD / OVERLAY
    pub cloud_vpcs: Vec<CloudVPC>,
    pub overlay_networks: Vec<OverlayNetwork>,
    pub kubernetes_services: Vec<K8sService>,

    // METADATA
    pub capture_duration_sec: f32,
    pub total_bytes_captured: u64,
    pub processing_notes: Vec<String>,
}

// ── HOST / DEVICE TYPES ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkHost {
    pub host_id: u64,
    pub ip_addresses: Vec<String>,
    pub mac_address: Option<String>,
    pub hostname: Option<String>,
    pub fqdn: Option<String>,
    pub os_guess: Option<OSFingerprint>,
    pub open_ports: Vec<OpenPort>,
    pub host_type: HostType,
    pub uptime_sec: Option<u64>,
    pub last_seen: f64,
    pub geo_location: Option<[f64; 2]>,      // lat, lon (for internet topology)
    pub asn: Option<u32>,
    pub org: Option<String>,
    pub tags: Vec<String>,
    pub is_internal: bool,
    pub criticality: HostCriticality,
    pub subnet_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OSFingerprint {
    pub os_family: String,              // "Linux", "Windows", "FreeBSD", "Cisco IOS", etc.
    pub os_version: Option<String>,
    pub cpe: Option<String>,            // Common Platform Enumeration
    pub method: String,                 // "nmap-tcp", "p0f", "DHCP", "configured"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenPort {
    pub port: u16,
    pub protocol: TransportProtocol,
    pub state: PortState,
    pub service_name: Option<String>,
    pub service_version: Option<String>,
    pub banner: Option<String>,
    pub vulnerability_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TransportProtocol { #[default] TCP, UDP, SCTP, DCCP, ICMP }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PortState { #[default] Open, Closed, Filtered, OpenFiltered, Unfiltered }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HostType { #[default] Unknown, Server, Workstation, Router, Switch, Firewall, Printer, IoT, Phone, Tablet, LoadBalancer, Hypervisor, Container, VirtualMachine, Custom(String) }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HostCriticality { #[default] Unknown, Low, Medium, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Subnet {
    pub subnet_id: u64,
    pub cidr: String,
    pub gateway: Option<String>,
    pub vlan_id: Option<u16>,
    pub name: Option<String>,
    pub host_count: u32,
    pub security_zone_id: Option<u64>,
    pub is_dmz: bool,
    pub ip_version: IPVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum IPVersion { #[default] IPv4, IPv6, DualStack }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Router {
    pub router_id: u64,
    pub host_id: u64,
    pub routing_protocols: Vec<RoutingProtocol>,
    pub interface_count: u32,
    pub bgp_as: Option<u32>,
    pub connected_subnets: Vec<String>,
    pub forwarding_table_size: Option<u32>,
    pub vendor: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RoutingProtocol { #[default] Static, OSPF, OSPFv3, IS_IS, BGP, EIGRP, RIP, RIPng, MPLS_LDP, MPLS_RSVP }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Switch {
    pub switch_id: u64,
    pub host_id: u64,
    pub port_count: u32,
    pub vlan_count: u32,
    pub spanning_tree_role: Option<String>,
    pub vendor: Option<String>, pub model: Option<String>,
    pub is_l3: bool,
    pub stacking_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FirewallDevice {
    pub fw_id: u64,
    pub host_id: u64,
    pub rule_count: u32,
    pub vendor: Option<String>, pub model: Option<String>,
    pub inspection_type: Vec<FirewallInspectionType>,
    pub protected_zones: Vec<u64>,
    pub nat_rules: u32,
    pub vpn_tunnels: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FirewallInspectionType { #[default] Stateful, Stateless, DPI, AppAware, TLS_Inspection }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoadBalancer {
    pub lb_id: u64,
    pub host_id: u64,
    pub algorithm: LBAlgorithm,
    pub virtual_ips: Vec<String>,
    pub backend_pool_size: u32,
    pub protocol: Vec<String>,
    pub health_check_type: Option<String>,
    pub vendor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LBAlgorithm { #[default] RoundRobin, LeastConnections, IPHash, WeightedRR, ECMP, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WirelessAP {
    pub ap_id: u64,
    pub host_id: u64,
    pub ssids: Vec<String>,
    pub channel_2_4ghz: Option<u8>,
    pub channel_5ghz: Option<u8>,
    pub channel_6ghz: Option<u8>,
    pub security: Vec<WirelessSecurity>,
    pub client_count: u32,
    pub vendor: Option<String>,
    pub location: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum WirelessSecurity { #[default] Open, WEP, WPA, WPA2_Personal, WPA2_Enterprise, WPA3, Custom(String) }

// ── SERVICE TYPES ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkService {
    pub service_id: u64,
    pub host_id: u64,
    pub name: String,
    pub port: u16,
    pub protocol: TransportProtocol,
    pub application_protocol: ApplicationProtocol,
    pub version: Option<String>,
    pub is_exposed_externally: bool,
    pub upstream_services: Vec<u64>,
    pub downstream_services: Vec<u64>,
    pub tls_enabled: bool,
    pub auth_required: bool,
    pub health_status: ServiceHealthStatus,
    pub avg_response_ms: Option<f32>,
    pub request_rate_per_sec: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ApplicationProtocol {
    #[default] Unknown,
    HTTP, HTTPS, HTTP2, HTTP3, gRPC, WebSocket,
    SSH, Telnet, FTP, SFTP, FTPS,
    SMTP, IMAP, POP3, LMTP,
    DNS, MDNS, LLMNR,
    SNMP, SYSLOG, NTP,
    LDAP, LDAPS, Kerberos, RADIUS,
    RDP, VNC, X11,
    MySQL, PostgreSQL, MongoDB, Redis, Cassandra, Elasticsearch,
    Kafka, RabbitMQ, MQTT, AMQP,
    NFS, SMB, iSCSI,
    Kubernetes_API, Docker_API, gRPC_Health,
    OSPF, BGP, EIGRP, SNMP_Trap,
    IPSec, OpenVPN, WireGuard,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ServiceHealthStatus { #[default] Unknown, Healthy, Degraded, Unhealthy, Unreachable }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceDependency {
    pub dep_id: u64,
    pub upstream_service_id: u64,
    pub downstream_service_id: u64,
    pub dependency_type: ServiceDependencyType,
    pub is_critical: bool,
    pub avg_latency_ms: Option<f32>,
    pub call_rate_per_sec: Option<f32>,
    pub error_rate: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ServiceDependencyType {
    #[default] SynchronousHTTP, SynchronousgRPC, AsynchronousQueue, DatabaseRead,
    DatabaseWrite, CacheRead, CacheWrite, DNS_Resolution, Authentication, Custom(String),
}

// ── FLOW TYPES ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowPath {
    pub flow_id: u64,
    pub src_ip: String, pub dst_ip: String,
    pub src_port: Option<u16>, pub dst_port: Option<u16>,
    pub protocol: TransportProtocol,
    pub application_protocol: Option<ApplicationProtocol>,
    pub bytes: u64, pub packets: u64,
    pub duration_sec: f32,
    pub start_time: f64, pub end_time: f64,
    pub hops: Vec<String>,              // IP addresses along path
    pub flags: Vec<TCPFlag>,
    pub is_encrypted: bool,
    pub flow_direction: FlowDirection,
    pub anomaly_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TCPFlag { #[default] SYN, ACK, FIN, RST, PSH, URG, ECE, CWR }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FlowDirection { #[default] Inbound, Outbound, Lateral, Internal, External }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopTalker {
    pub talker_id: u64,
    pub ip_address: String,
    pub total_bytes: u64, pub total_packets: u64,
    pub total_flows: u32,
    pub top_destinations: Vec<String>,
    pub top_protocols: Vec<String>,
    pub bytes_in: u64, pub bytes_out: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolStat {
    pub protocol: String,
    pub flow_count: u32,
    pub byte_count: u64,
    pub packet_count: u64,
    pub fraction_of_total: f32,
}

// ── ROUTING TYPES ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingEntry {
    pub entry_id: u64,
    pub prefix: String,
    pub next_hop: String,
    pub metric: u32,
    pub protocol: RoutingProtocol,
    pub origin_router_id: Option<u64>,
    pub age_sec: Option<u32>,
    pub is_best: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ASPath {
    pub path_id: u64,
    pub prefix: String,
    pub as_sequence: Vec<u32>,          // ordered list of ASNs
    pub origin_as: u32,
    pub next_hop_ip: String,
    pub local_preference: Option<u32>,
    pub med: Option<u32>,
    pub communities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BGPPeer {
    pub peer_id: u64,
    pub local_ip: String, pub peer_ip: String,
    pub local_as: u32, pub peer_as: u32,
    pub state: BGPSessionState,
    pub prefixes_received: u32,
    pub prefixes_sent: u32,
    pub uptime_sec: u64,
    pub session_type: BGPSessionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BGPSessionState { #[default] Established, Active, Idle, Connect, OpenSent, OpenConfirm }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BGPSessionType { #[default] EBGP, IBGP, RouteReflector, Confederation }

// ── SEGMENTATION / ZONE TYPES ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VLAN {
    pub vlan_id: u16,
    pub name: Option<String>,
    pub subnet_ids: Vec<u64>,
    pub host_count: u32,
    pub tagged_ports: Vec<String>,
    pub untagged_ports: Vec<String>,
    pub is_management: bool,
    pub is_voice: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityZone {
    pub zone_id: u64,
    pub name: String,
    pub trust_level: TrustLevel,
    pub subnet_ids: Vec<u64>,
    pub host_ids: Vec<u64>,
    pub inbound_policies: Vec<String>,
    pub outbound_policies: Vec<String>,
    pub is_dmz: bool,
    pub is_internet_facing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TrustLevel { #[default] Unknown, Untrusted, Restricted, Internal, Trusted, Management }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkSegment {
    pub segment_id: u64,
    pub name: String,
    pub segment_type: SegmentType,
    pub host_count: u32,
    pub service_count: u32,
    pub boundary_devices: Vec<u64>,     // routers/firewalls at segment boundary
    pub is_isolated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SegmentType { #[default] Unknown, Corporate, Guest, IoT, OT, DMZ, Management, Cloud, Custom(String) }

// ── TOPOLOGY / RESILIENCE ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TopologyType { #[default] Unknown, Bus, Star, Ring, Mesh, FullMesh, PartialMesh, Tree, Hybrid, FatTree, SpineLeaf, Point_to_Point }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResilienceMetrics {
    pub node_connectivity: u32,         // min nodes to disconnect graph
    pub edge_connectivity: u32,         // min edges to disconnect graph
    pub diameter: u32,                  // longest shortest path
    pub avg_path_length: f64,
    pub clustering_coefficient: f64,
    pub betweenness_centrality_top: Vec<(u64, f64)>,  // (node_id, score)
    pub single_points_of_failure: Vec<u64>,
    pub redundant_path_count: HashMap<String, u32>,   // "src→dst" → count
}

// ── ANOMALY TYPES ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkAnomaly {
    pub anomaly_id: u64,
    pub anomaly_type: AnomalyType,
    pub description: String,
    pub affected_host_ids: Vec<u64>,
    pub affected_service_ids: Vec<u64>,
    pub severity: AnomalySeverity,
    pub detected_at: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AnomalySeverity { #[default] Info, Low, Medium, High, Critical }

// ── CLOUD / OVERLAY / K8s ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloudVPC {
    pub vpc_id: u64,
    pub cloud_provider: String,
    pub vpc_identifier: String,
    pub region: String,
    pub cidr_blocks: Vec<String>,
    pub subnet_count: u32,
    pub host_count: u32,
    pub has_internet_gateway: bool,
    pub peering_connections: Vec<String>,
    pub security_group_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverlayNetwork {
    pub overlay_id: u64,
    pub overlay_type: OverlayType,
    pub name: String,
    pub underlay_host_count: u32,
    pub virtual_node_count: u32,
    pub encapsulation: EncapsulationType,
    pub control_plane: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OverlayType { #[default] Unknown, VXLAN, GENEVE, GRE, IPinIP, WireGuard, OpenVPN, ServiceMesh }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EncapsulationType { #[default] Unknown, VXLAN, GENEVE, GRE, IPIP, MPLS, TLS }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct K8sService {
    pub service_id: u64,
    pub namespace: String,
    pub name: String,
    pub service_type: K8sServiceType,
    pub cluster_ip: Option<String>,
    pub external_ip: Option<String>,
    pub ports: Vec<K8sPort>,
    pub selector_labels: HashMap<String, String>,
    pub pod_count: u32,
    pub endpoint_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum K8sServiceType { #[default] ClusterIP, NodePort, LoadBalancer, ExternalName, Headless }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct K8sPort { pub name: Option<String>, pub port: u16, pub target_port: u16, pub protocol: TransportProtocol }

// ── NETWORK UPDATE ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkUpdate {
    AddHost { host: NetworkHost },
    RemoveHost { host_ip: String },
    UpdateHostStatus { host_id: u64, new_type: HostType, criticality: HostCriticality },
    AddService { service: NetworkService },
    RemoveService { service_id: u64 },
    AddFlow { flow: FlowPath },
    UpdateSecurityZone { zone_id: u64, trust_level: TrustLevel },
    AddAnomaly { anomaly: NetworkAnomaly },
    UpdateRoutingEntry { entry: RoutingEntry },
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH NODE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum NetworkNodeType {
    #[default] NetworkScene,            // root
    // HOSTS / DEVICES
    HostNode, RouterNode, SwitchNode, FirewallNode, LoadBalancerNode, WirelessAPNode,
    // NETWORK STRUCTURE
    SubnetNode, VLANNode, SecurityZoneNode, NetworkSegmentNode,
    // SERVICES
    ServiceNode, ServiceDependencyNode,
    // FLOWS
    FlowPathNode, TopTalkerNode,
    // ROUTING
    RoutingEntryNode, ASPathNode, BGPPeerNode,
    // CLOUD / OVERLAY
    CloudVPCNode, OverlayNetworkNode, K8sServiceNode,
    // ANOMALY
    AnomalyNode,
    // CROSS-MODAL
    CrossModalRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkGraphNode {
    pub node_id: u64,
    pub node_type: NetworkNodeType,
    pub content: String,

    // NETWORK-SPECIFIC
    pub ip_address: Option<String>,
    pub port: Option<u16>,
    pub asn: Option<u32>,
    pub cidr: Option<String>,
    pub host_type: Option<String>,
    pub protocol: Option<String>,
    pub bytes: Option<u64>,
    pub geo_location: Option<[f64; 2]>,
    pub is_internal: Option<bool>,
    pub criticality: Option<String>,
    pub service_name: Option<String>,
    pub health_status: Option<String>,

    // UNIVERSAL NODE FIELDS
    pub provisional: bool,
    pub provisional_status: ProvisionalStatus,
    pub materialized_path: Option<String>,
    pub created_by_step: Option<u32>,
    pub updated_by_step: Option<u32>,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
    pub keywords: Vec<String>,
    pub embedding_hint: Option<String>,
    pub hotness_score: f32,
    pub source_chunk_index: Option<u32>,
    pub source_start_char: Option<usize>,
    pub source_end_char: Option<usize>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH EDGE TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum NetworkEdgeType {
    // ── STRUCTURAL ──
    #[default] Contains, Precedes, PartOf,

    // ── NETWORK-SPECIFIC ──
    TopologicallyConnected,     // layer 1/2/3 physical/logical connection
    TransmitsTo,                // flow from A to B
    RoutedThrough,              // traffic routed through a device
    ConnectedViaSwitch,         // hosts connected through same switch
    ConnectedViaRouter,         // networks connected via router
    FirewalledBy,               // traffic controlled by firewall
    LoadBalancedBy,             // service fronted by load balancer
    InSubnet,                   // host is in subnet
    InVLAN,                     // host/subnet is in VLAN
    InSecurityZone,             // host/subnet in security zone
    BGPPeersWith,               // BGP session between routers
    AdvertisesPrefix,           // router advertises routing prefix
    ServesOver,                 // service runs on host:port
    DependsOnService,           // service depends on another service
    ResolvedBy,                 // hostname resolved by DNS service
    AuthenticatedBy,            // service auth via auth service
    ForwardedBy,                // flow forwarded through hop
    TunneledThrough,            // traffic tunneled through overlay
    EncapsulatedIn,             // overlay encapsulates underlay
    MirroredBy,                 // traffic mirrored to a tap/monitor
    BlockedBy,                  // connection blocked by firewall
    NAT_ted_To,                 // address translated to another

    // ── CROSS-MODAL ──
    /// Host placed in 3D building layout (109)
    PlacedIn3D,
    /// Network node geo-referenced (117)
    GeoReferenced,
    /// RF physical layer (118)
    CarriedByEM,
    /// Service implemented by code (101)
    ImplementedByCode,
    /// Links to control system (122)
    ControlPlaneOf,

    // ── UNIVERSAL SEMANTIC ──
    Performs, Affects, Implies, Contradicts, Elaborates, Summarizes, Supports,
    TemporalPrecedes, TemporalFollows, CausedBy, Enables, Prevents,
    FunctionalRole, InstanceOf,
    DerivedFrom, VersionOf, RefinesTo, ForkedFrom, SimilarTo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkGraphEdge {
    pub edge_id: u64,
    pub from_node: u64, pub to_node: u64,
    pub edge_type: NetworkEdgeType,
    pub weight: f32,
    pub provenance: EdgeProvenance,
    pub created_by_step: Option<u32>,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
    pub properties: HashMap<String, serde_json::Value>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH STRUCTURE
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkGraph {
    pub graph_id: u64, pub project_id: u64,
    pub source_description: String,
    pub nodes: Vec<NetworkGraphNode>,
    pub edges: Vec<NetworkGraphEdge>,
    pub root_node_id: u64,
    pub state: GraphStateType,
    pub state_history: Vec<GraphStateTransition>,
    pub created_at: String, pub updated_at: String,
    pub version: u32, pub version_notes: Vec<VersionNote>,
}

// ─────────────────────────────────────────────────────────────────────────────
// QUERY / HOOKS / DISPLAY
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkGraphQuery {
    NodeDetail { node_id: u64 },
    HostsByType { host_type: String },
    ServicesByProtocol { protocol: String },
    HostsInSubnet { cidr: String },
    HostsInSecurityZone { zone_id: u64 },
    FlowsBetween { src_ip: String, dst_ip: String },
    AnomaliesBySeverity { min_severity: String },
    BGPPeers,
    CriticalHosts,
    ServiceDependencies { service_id: u64 },
    CrossModalLinks { node_id: u64 },
    AGIActivity, AllNodes, AllEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkSemanticHook {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink { target_modality: String, target_graph_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkExportFormat {
    GraphML,            // standard graph format
    Cytoscape_JSON,     // Cytoscape network visualization
    D3_JSON,            // D3.js force graph
    GEXF,               // Gephi format
    GeoJSON,            // geo-referenced nodes
    CSV_Edgelist,       // simple edge list
    DOT,                // Graphviz
    Nmap_XML,           // nmap-compatible host list
    NetFlow_CSV,        // flow export
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkDisplayMode {
    TopologyMap,            // force-directed or hierarchical topology
    SubnetHeatmap,          // subnet traffic heat map
    FlowSankey,             // flow volumes as Sankey
    ServiceDependencyGraph, // service dependency map
    GeoMap,                 // hosts on geographic map
    SecurityZoneView,       // zone-based segmentation
    TimelineView,           // temporal flow/anomaly timeline
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkOperation {
    RefreshHostInventory,
    RecomputePaths,
    CrossLinkToGeo { geo_graph_id: u64 },
    CrossLinkToEM { em_graph_id: u64 },
    CrossLinkTo3D { three_d_graph_id: u64 },
    DetectSegmentation,
    ComputeResilienceMetrics,
    ExportTopologyFile,
}

// ─────────────────────────────────────────────────────────────────────────────
// OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkModalityOutput {
    pub success: bool,
    pub graph_id: Option<u64>,
    pub graph: Option<NetworkGraph>,
    pub analysis: Option<NetworkAnalysisResult>,
    pub discovered_hosts: Option<Vec<NetworkHost>>,
    pub flow_paths: Option<Vec<FlowPath>>,
    pub resilience_metrics: Option<ResilienceMetrics>,
    pub path_analysis: Option<Vec<Vec<u64>>>,   // list of node-id paths
    pub anomalies: Option<Vec<NetworkAnomaly>>,
    pub segments: Option<Vec<NetworkSegment>>,
    pub query_result: Option<serde_json::Value>,
    pub export_path: Option<String>,
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SHARED TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProvisionalStatus { #[default] Planned, Generating, Generated, Validated, Finalized, Failed, RolledBack }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionNote { pub version: u32, pub note: String, pub step_index: Option<u32>, pub timestamp: String, pub change_type: ChangeType }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType { #[default] Created, Updated, CrossLinked, EnrichedBySemantic, EnrichedByHook, RolledBack, Finalized }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeProvenance {
    #[default] Unknown, DerivedFromPrompt, DerivedFromChunk(u32), DerivedFromChunkGraph(u64),
    DerivedFromModalityGraph(u64), DerivedFromFile(String),
    DerivedFromAMT, DerivedFromBlueprint(u32), DerivedFromMethodology(u64),
    DerivedFromCrossModal, DerivedFromHook, VersionOf(u32), ForkedFrom(u64),
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GraphStateType { #[default] Created, SemanticEnriched, CrossLinked, Stable, Updated, ReValidating, Archived }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStateTransition { pub from: GraphStateType, pub to: GraphStateType, pub timestamp: String, pub triggered_by_step: Option<u32> }

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE EXECUTOR
// ─────────────────────────────────────────────────────────────────────────────

struct PipelineExecutor { zsei_path: String, prompt_pipeline_path: String }

impl PipelineExecutor {
    fn new() -> Self {
        Self {
            zsei_path: env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".into()),
            prompt_pipeline_path: env::var("OZONE_PROMPT_PIPELINE").unwrap_or_else(|_| "./pipeline_9".into()),
        }
    }

    async fn llm_zero_shot(&self, prompt: &str, max_tokens: usize) -> Result<String, String> {
        let input = serde_json::json!({"action":"Prompt","prompt":prompt,"max_tokens":max_tokens,"temperature":0.05,"system_context":"Network topology analysis. Return only valid JSON."});
        let out = std::process::Command::new(&self.prompt_pipeline_path).arg("--input").arg(input.to_string()).output().map_err(|e| e.to_string())?;
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
        Ok(r["response"].as_str().unwrap_or("{}").to_string())
    }

    fn save_graph(&self, g: &NetworkGraph) -> Result<(), String> {
        let path = format!("{}/local/net_graph_{}.json", self.zsei_path, g.graph_id);
        if let Some(p) = std::path::Path::new(&path).parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, serde_json::to_string_pretty(g).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn load_graph(&self, id: u64) -> Result<NetworkGraph, String> {
        let path = format!("{}/local/net_graph_{}.json", self.zsei_path, id);
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn generate_id(&self) -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
    fn now_iso8601(&self) -> String { format!("{}", self.generate_id()) }
    fn extract_json_array(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('['), raw.rfind(']')) { raw[s..=e].to_string() } else { "[]".to_string() } }
    fn extract_json_object(raw: &str) -> String { if let (Some(s), Some(e)) = (raw.find('{'), raw.rfind('}')) { raw[s..=e].to_string() } else { "{}".to_string() } }
}

impl PipelineExecutor {
    async fn classify_hosts_llm(&self, hosts: &[NetworkHost]) -> Vec<(u64, HostType, HostCriticality)> {
        if hosts.is_empty() { return vec![]; }
        let host_list: Vec<serde_json::Value> = hosts.iter().take(25).map(|h| serde_json::json!({
            "host_id": h.host_id,
            "ips": h.ip_addresses,
            "hostname": h.hostname,
            "os": h.os_guess.as_ref().map(|o| &o.os_family),
            "open_ports": h.open_ports.iter().map(|p| serde_json::json!({"port": p.port, "svc": p.service_name})).collect::<Vec<_>>(),
            "is_internal": h.is_internal,
        })).collect();

        let prompt = format!(r#"
Classify each network host based on its characteristics.

Hosts:
{}

Host type options: Server, Workstation, Router, Switch, Firewall, Printer, IoT, Phone, LoadBalancer, Hypervisor, Container
Criticality options: Low, Medium, High, Critical

Consider: servers with many open ports are High/Critical; IoT devices are typically Low;
infrastructure devices (routers/switches/firewalls) are Critical.

Return ONLY valid JSON array:
[{{"host_id": N, "host_type": "TypeName", "criticality": "Level"}}]"#,
            serde_json::to_string_pretty(&host_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let id = v["host_id"].as_u64()?;
                        let ht = match v["host_type"].as_str().unwrap_or("Unknown") {
                            "Server" => HostType::Server, "Workstation" => HostType::Workstation,
                            "Router" => HostType::Router, "Switch" => HostType::Switch,
                            "Firewall" => HostType::Firewall, "Printer" => HostType::Printer,
                            "IoT" => HostType::IoT, "Phone" => HostType::Phone,
                            "LoadBalancer" => HostType::LoadBalancer,
                            "Hypervisor" => HostType::Hypervisor,
                            _ => HostType::Unknown,
                        };
                        let crit = match v["criticality"].as_str().unwrap_or("Unknown") {
                            "Low" => HostCriticality::Low, "Medium" => HostCriticality::Medium,
                            "High" => HostCriticality::High, "Critical" => HostCriticality::Critical,
                            _ => HostCriticality::Unknown,
                        };
                        Some((id, ht, crit))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn identify_security_zones_llm(&self, subnets: &[Subnet], hosts: &[NetworkHost]) -> Vec<(u64, String, TrustLevel)> {
        if subnets.is_empty() { return vec![]; }
        let sn_list: Vec<serde_json::Value> = subnets.iter().map(|s| serde_json::json!({
            "subnet_id": s.subnet_id, "cidr": s.cidr, "name": s.name,
            "is_dmz": s.is_dmz, "vlan_id": s.vlan_id, "host_count": s.host_count,
        })).collect();

        let prompt = format!(r#"
Classify each network subnet into a security zone based on its characteristics.

Subnets:
{}

Trust levels: Untrusted (internet/external), Restricted (DMZ/guest), Internal (corporate), Trusted (datacenter/server), Management (admin)

Return ONLY valid JSON array:
[{{"subnet_id": N, "zone_name": "ZoneName", "trust_level": "Level"}}]"#,
            serde_json::to_string_pretty(&sn_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 400).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let id = v["subnet_id"].as_u64()?;
                        let name = v["zone_name"].as_str().unwrap_or("Unknown").to_string();
                        let trust = match v["trust_level"].as_str().unwrap_or("Unknown") {
                            "Untrusted" => TrustLevel::Untrusted, "Restricted" => TrustLevel::Restricted,
                            "Internal" => TrustLevel::Internal, "Trusted" => TrustLevel::Trusted,
                            "Management" => TrustLevel::Management, _ => TrustLevel::Unknown,
                        };
                        Some((id, name, trust))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    async fn infer_semantic_relationships(&self, nodes: &[NetworkGraphNode]) -> Vec<(u64, u64, NetworkEdgeType, String)> {
        if nodes.len() < 2 { return vec![]; }
        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
            "node_id": n.node_id, "type": format!("{:?}", n.node_type),
            "content": n.content.chars().take(80).collect::<String>(),
            "ip": n.ip_address, "port": n.port, "protocol": n.protocol,
        })).collect();

        let prompt = format!(r#"
Identify semantic network relationships between these nodes not already captured structurally.

Nodes: {}

Types: TopologicallyConnected, TransmitsTo, RoutedThrough, FirewalledBy, DependsOnService,
ResolvedBy, AuthenticatedBy, BGPPeersWith, InSubnet, ForwardedBy, TunneledThrough,
Affects, CausedBy, Enables, DerivedFrom, SimilarTo

Return ONLY valid JSON array:
[{{"from_node_id": N, "to_node_id": M, "edge_type": "TypeName", "reason": "brief"}}]"#,
            serde_json::to_string_pretty(&node_list).unwrap_or_default());

        match self.llm_zero_shot(&prompt, 600).await {
            Ok(raw) => {
                let json_str = Self::extract_json_array(&raw);
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str).unwrap_or_default().into_iter()
                    .filter_map(|v| {
                        let from = v["from_node_id"].as_u64()?;
                        let to = v["to_node_id"].as_u64()?;
                        let etype = map_net_edge_str(v["edge_type"].as_str().unwrap_or("Affects"));
                        let reason = v["reason"].as_str().unwrap_or("").to_string();
                        Some((from, to, etype, reason))
                    }).collect()
            }
            Err(_) => vec![],
        }
    }

    // Breadth-first shortest path between two node IDs in the graph
    fn find_shortest_path(nodes: &[NetworkGraphNode], edges: &[NetworkGraphEdge], src: u64, dst: u64) -> Option<Vec<u64>> {
        use std::collections::VecDeque;
        let valid: std::collections::HashSet<u64> = nodes.iter().map(|n| n.node_id).collect();
        if !valid.contains(&src) || !valid.contains(&dst) { return None; }

        // Build adjacency list
        let mut adj: HashMap<u64, Vec<u64>> = HashMap::new();
        for e in edges {
            adj.entry(e.from_node).or_default().push(e.to_node);
            adj.entry(e.to_node).or_default().push(e.from_node);
        }

        let mut visited: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut queue: VecDeque<Vec<u64>> = VecDeque::new();
        queue.push_back(vec![src]);
        visited.insert(src);

        while let Some(path) = queue.pop_front() {
            let last = *path.last().unwrap();
            if last == dst { return Some(path); }
            if let Some(neighbors) = adj.get(&last) {
                for &nbr in neighbors {
                    if !visited.contains(&nbr) {
                        visited.insert(nbr);
                        let mut new_path = path.clone();
                        new_path.push(nbr);
                        queue.push_back(new_path);
                    }
                }
            }
        }
        None
    }

    // Compute basic resilience metrics
    fn compute_resilience_metrics(nodes: &[NetworkGraphNode], edges: &[NetworkGraphEdge]) -> ResilienceMetrics {
        let n = nodes.len();
        if n == 0 { return ResilienceMetrics::default(); }

        // Build adjacency for path length computation
        let mut adj: HashMap<u64, Vec<u64>> = HashMap::new();
        for e in edges {
            adj.entry(e.from_node).or_default().push(e.to_node);
            adj.entry(e.to_node).or_default().push(e.from_node);
        }

        // Compute degree for betweenness (simplified: use degree centrality)
        let mut degree: HashMap<u64, usize> = HashMap::new();
        for e in edges {
            *degree.entry(e.from_node).or_insert(0) += 1;
            *degree.entry(e.to_node).or_insert(0) += 1;
        }

        let mut top_betweenness: Vec<(u64, f64)> = degree.iter()
            .map(|(&nid, &d)| (nid, d as f64 / n.max(1) as f64))
            .collect();
        top_betweenness.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        top_betweenness.truncate(5);

        // Identify nodes with degree 1 + bridge heuristic (articulation point proxy)
        let single_points: Vec<u64> = degree.iter()
            .filter(|(_, &d)| d == 1)
            .map(|(&id, _)| id)
            .collect();

        let total_edges = edges.len();
        let avg_degree = if n > 0 { (2 * total_edges) as f64 / n as f64 } else { 0.0 };
        let max_possible_edges = (n * (n - 1)) / 2;
        let clustering = if max_possible_edges > 0 { total_edges as f64 / max_possible_edges as f64 } else { 0.0 };

        ResilienceMetrics {
            node_connectivity: degree.values().copied().min().unwrap_or(0) as u32,
            edge_connectivity: degree.values().copied().min().unwrap_or(0) as u32,
            diameter: (n as f64).sqrt() as u32,   // heuristic
            avg_path_length: if avg_degree > 1.0 { (n as f64).log(avg_degree) } else { n as f64 },
            clustering_coefficient: clustering,
            betweenness_centrality_top: top_betweenness,
            single_points_of_failure: single_points,
            redundant_path_count: HashMap::new(),
        }
    }
}

fn map_net_edge_str(s: &str) -> NetworkEdgeType {
    match s {
        "TopologicallyConnected"  => NetworkEdgeType::TopologicallyConnected,
        "TransmitsTo"             => NetworkEdgeType::TransmitsTo,
        "RoutedThrough"           => NetworkEdgeType::RoutedThrough,
        "ConnectedViaSwitch"      => NetworkEdgeType::ConnectedViaSwitch,
        "ConnectedViaRouter"      => NetworkEdgeType::ConnectedViaRouter,
        "FirewalledBy"            => NetworkEdgeType::FirewalledBy,
        "LoadBalancedBy"          => NetworkEdgeType::LoadBalancedBy,
        "InSubnet"                => NetworkEdgeType::InSubnet,
        "InVLAN"                  => NetworkEdgeType::InVLAN,
        "InSecurityZone"          => NetworkEdgeType::InSecurityZone,
        "BGPPeersWith"            => NetworkEdgeType::BGPPeersWith,
        "AdvertisesPrefix"        => NetworkEdgeType::AdvertisesPrefix,
        "ServesOver"              => NetworkEdgeType::ServesOver,
        "DependsOnService"        => NetworkEdgeType::DependsOnService,
        "ResolvedBy"              => NetworkEdgeType::ResolvedBy,
        "AuthenticatedBy"         => NetworkEdgeType::AuthenticatedBy,
        "ForwardedBy"             => NetworkEdgeType::ForwardedBy,
        "TunneledThrough"         => NetworkEdgeType::TunneledThrough,
        "BlockedBy"               => NetworkEdgeType::BlockedBy,
        "PlacedIn3D"              => NetworkEdgeType::PlacedIn3D,
        "GeoReferenced"           => NetworkEdgeType::GeoReferenced,
        "CarriedByEM"             => NetworkEdgeType::CarriedByEM,
        "ImplementedByCode"       => NetworkEdgeType::ImplementedByCode,
        "ControlPlaneOf"          => NetworkEdgeType::ControlPlaneOf,
        "Affects"                 => NetworkEdgeType::Affects,
        "CausedBy"                => NetworkEdgeType::CausedBy,
        "Enables"                 => NetworkEdgeType::Enables,
        "TemporalPrecedes"        => NetworkEdgeType::TemporalPrecedes,
        "DerivedFrom"             => NetworkEdgeType::DerivedFrom,
        "PartOf"                  => NetworkEdgeType::PartOf,
        "SimilarTo"               => NetworkEdgeType::SimilarTo,
        _                         => NetworkEdgeType::Affects,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH CREATION
// ─────────────────────────────────────────────────────────────────────────────

async fn create_graph(executor: &PipelineExecutor, analysis: NetworkAnalysisResult, project_id: u64) -> NetworkModalityOutput {
    let graph_id = executor.generate_id();
    let now = executor.now_iso8601();
    let mut nodes: Vec<NetworkGraphNode> = Vec::new();
    let mut edges: Vec<NetworkGraphEdge> = Vec::new();
    let mut node_id: u64 = 1;
    let mut edge_id: u64 = 1;

    // ── ROOT ──
    let root_id = node_id;
    nodes.push(NetworkGraphNode {
        node_id: root_id, node_type: NetworkNodeType::NetworkScene,
        content: format!("Network [{}]: hosts={} services={} flows={} subnets={} anomalies={}",
            format!("{:?}", analysis.topology_type),
            analysis.total_host_count, analysis.total_service_count, analysis.total_flow_count,
            analysis.subnets.len(), analysis.anomalies.len()),
        materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}", project_id, graph_id)),
        provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
        version_notes: vec![VersionNote { version: 1, note: "Initial creation".into(), step_index: None, timestamp: now.clone(), change_type: ChangeType::Created }],
        keywords: vec!["network".into(), "topology".into(), format!("{:?}", analysis.topology_type).to_lowercase()],
        hotness_score: 1.0, ..Default::default()
    });
    node_id += 1;

    // ── SUBNET NODES ──
    let mut subnet_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for sn in &analysis.subnets {
        let snid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: snid, node_type: NetworkNodeType::SubnetNode,
            content: format!("Subnet {} {}: hosts={} gw={:?} vlan={:?} dmz={}",
                sn.name.as_deref().unwrap_or("?"), sn.cidr,
                sn.host_count, sn.gateway, sn.vlan_id, sn.is_dmz),
            cidr: Some(sn.cidr.clone()),
            is_internal: Some(!sn.is_dmz),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Subnet/{}", project_id, graph_id, sn.subnet_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: { let mut kw = vec!["subnet".into(), sn.cidr.clone()]; if let Some(ref n) = sn.name { kw.push(n.to_lowercase()); } kw },
            hotness_score: 0.7, ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: snid, edge_type: NetworkEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        subnet_id_to_nid.insert(sn.subnet_id, snid);
        node_id += 1;
    }

    // ── VLAN NODES ──
    let mut vlan_id_to_nid: HashMap<u16, u64> = HashMap::new();
    for vlan in &analysis.vlans {
        let vnid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: vnid, node_type: NetworkNodeType::VLANNode,
            content: format!("VLAN {}: {} hosts={} mgmt={} voice={}",
                vlan.vlan_id, vlan.name.as_deref().unwrap_or("?"), vlan.host_count, vlan.is_management, vlan.is_voice),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/VLAN/{}", project_id, graph_id, vlan.vlan_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: { let mut kw = vec!["vlan".into(), format!("vlan{}", vlan.vlan_id)]; if let Some(ref n) = vlan.name { kw.push(n.to_lowercase()); } kw },
            hotness_score: 0.6, ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: vnid, edge_type: NetworkEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // VLAN → subnets
        for sn_id in &vlan.subnet_ids {
            if let Some(&sn_nid) = subnet_id_to_nid.get(sn_id) {
                edges.push(NetworkGraphEdge { edge_id, from_node: vnid, to_node: sn_nid, edge_type: NetworkEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        vlan_id_to_nid.insert(vlan.vlan_id, vnid);
        node_id += 1;
    }

    // ── SECURITY ZONE NODES ──
    let mut zone_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for zone in &analysis.security_zones {
        let znid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: znid, node_type: NetworkNodeType::SecurityZoneNode,
            content: format!("Zone: {} [{:?}] subnets={} dmz={} internet={}",
                zone.name, zone.trust_level, zone.subnet_ids.len(), zone.is_dmz, zone.is_internet_facing),
            is_internal: Some(zone.trust_level == TrustLevel::Internal || zone.trust_level == TrustLevel::Trusted),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Zone/{}", project_id, graph_id, zone.zone_id)),
            provisional: false, provisional_status: ProvisionalStatus::Finalized, version: 1,
            keywords: vec!["security-zone".into(), zone.name.to_lowercase(), format!("{:?}", zone.trust_level).to_lowercase()],
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: znid, edge_type: NetworkEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Zone → subnets
        for sn_id in &zone.subnet_ids {
            if let Some(&sn_nid) = subnet_id_to_nid.get(sn_id) {
                edges.push(NetworkGraphEdge { edge_id, from_node: sn_nid, to_node: znid, edge_type: NetworkEdgeType::InSecurityZone, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        zone_id_to_nid.insert(zone.zone_id, znid);
        node_id += 1;
    }

    // ── HOST NODES ──
    let mut host_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for host in &analysis.hosts {
        let hnid = node_id;
        let primary_ip = host.ip_addresses.first().cloned().unwrap_or_default();
        nodes.push(NetworkGraphNode {
            node_id: hnid, node_type: NetworkNodeType::HostNode,
            content: format!("Host {:?}/{:?}: {} [{}] ports={} os={:?}",
                host.host_type, host.criticality,
                primary_ip, host.hostname.as_deref().unwrap_or("?"),
                host.open_ports.len(),
                host.os_guess.as_ref().map(|o| o.os_family.as_str()).unwrap_or("?")),
            ip_address: Some(primary_ip.clone()),
            host_type: Some(format!("{:?}", host.host_type)),
            criticality: Some(format!("{:?}", host.criticality)),
            geo_location: host.geo_location,
            asn: host.asn,
            is_internal: Some(host.is_internal),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Host/{}", project_id, graph_id, host.host_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: {
                let mut kw = vec!["host".into(), format!("{:?}", host.host_type).to_lowercase()];
                kw.push(primary_ip.clone());
                if let Some(ref hn) = host.hostname { kw.push(hn.to_lowercase()); }
                kw
            },
            hotness_score: match host.criticality { HostCriticality::Critical => 0.95, HostCriticality::High => 0.85, HostCriticality::Medium => 0.7, _ => 0.55 },
            ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: hnid, edge_type: NetworkEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Host → subnet
        if let Some(sn_id) = host.subnet_id {
            if let Some(&sn_nid) = subnet_id_to_nid.get(&sn_id) {
                edges.push(NetworkGraphEdge { edge_id, from_node: hnid, to_node: sn_nid, edge_type: NetworkEdgeType::InSubnet, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        // Cross-modal: geo-referenced host → geospatial (117)
        if host.geo_location.is_some() {
            edges.push(NetworkGraphEdge { edge_id, from_node: hnid, to_node: hnid, edge_type: NetworkEdgeType::GeoReferenced, weight: 0.85, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("geospatial")); p }, ..Default::default() });
            edge_id += 1;
        }
        host_id_to_nid.insert(host.host_id, hnid);
        node_id += 1;
    }

    // ── ROUTER NODES ──
    for router in &analysis.routers {
        if let Some(&host_nid) = host_id_to_nid.get(&router.host_id) {
            let rnid = node_id;
            nodes.push(NetworkGraphNode {
                node_id: rnid, node_type: NetworkNodeType::RouterNode,
                content: format!("Router: protocols={:?} bgp_as={:?} ifaces={} fwdtbl={:?}",
                    router.routing_protocols.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join(","),
                    router.bgp_as, router.interface_count, router.forwarding_table_size),
                asn: router.bgp_as,
                materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Router/{}", project_id, graph_id, router.router_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["router".into(), "routing".into()],
                hotness_score: 0.85, ..Default::default()
            });
            edges.push(NetworkGraphEdge { edge_id, from_node: host_nid, to_node: rnid, edge_type: NetworkEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1; node_id += 1;
        }
    }

    // ── SWITCH NODES ──
    for sw in &analysis.switches {
        if let Some(&host_nid) = host_id_to_nid.get(&sw.host_id) {
            let swnid = node_id;
            nodes.push(NetworkGraphNode {
                node_id: swnid, node_type: NetworkNodeType::SwitchNode,
                content: format!("Switch: ports={} vlans={} l3={} vendor={:?}",
                    sw.port_count, sw.vlan_count, sw.is_l3, sw.vendor.as_deref().unwrap_or("?")),
                materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Switch/{}", project_id, graph_id, sw.switch_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["switch".into(), "l2".into()], hotness_score: 0.75, ..Default::default()
            });
            edges.push(NetworkGraphEdge { edge_id, from_node: host_nid, to_node: swnid, edge_type: NetworkEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1; node_id += 1;
        }
    }

    // ── FIREWALL NODES ──
    for fw in &analysis.firewalls {
        if let Some(&host_nid) = host_id_to_nid.get(&fw.host_id) {
            let fwnid = node_id;
            nodes.push(NetworkGraphNode {
                node_id: fwnid, node_type: NetworkNodeType::FirewallNode,
                content: format!("Firewall: rules={} nat={} vpn={} vendor={:?}",
                    fw.rule_count, fw.nat_rules, fw.vpn_tunnels, fw.vendor.as_deref().unwrap_or("?")),
                materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Firewall/{}", project_id, graph_id, fw.fw_id)),
                provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
                keywords: vec!["firewall".into(), "security".into()], hotness_score: 0.9, ..Default::default()
            });
            edges.push(NetworkGraphEdge { edge_id, from_node: host_nid, to_node: fwnid, edge_type: NetworkEdgeType::Contains, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
            // Firewall protects zones
            for zone_id in &fw.protected_zones {
                if let Some(&zone_nid) = zone_id_to_nid.get(zone_id) {
                    edges.push(NetworkGraphEdge { edge_id, from_node: fwnid, to_node: zone_nid, edge_type: NetworkEdgeType::FirewalledBy, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                    edge_id += 1;
                }
            }
            node_id += 1;
        }
    }

    // ── SERVICE NODES ──
    let mut svc_id_to_nid: HashMap<u64, u64> = HashMap::new();
    for svc in &analysis.services {
        let svnid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: svnid, node_type: NetworkNodeType::ServiceNode,
            content: format!("Service: {} {:?}/{}:{} tls={} auth={} health={:?} rps={:?}",
                svc.name, svc.application_protocol, svc.protocol_str(), svc.port,
                svc.tls_enabled, svc.auth_required, svc.health_status, svc.request_rate_per_sec.map(|r| format!("{:.1}", r))),
            service_name: Some(svc.name.clone()),
            port: Some(svc.port),
            protocol: Some(format!("{:?}", svc.protocol)),
            health_status: Some(format!("{:?}", svc.health_status)),
            is_internal: Some(!svc.is_exposed_externally),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Service/{}", project_id, graph_id, svc.service_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: {
                let mut kw = vec!["service".into(), svc.name.to_lowercase(), format!("{:?}", svc.application_protocol).to_lowercase()];
                if svc.is_exposed_externally { kw.push("external".into()); }
                kw
            },
            hotness_score: if svc.is_exposed_externally { 0.85 } else { 0.65 },
            ..Default::default()
        });
        // Service → host
        if let Some(&h_nid) = host_id_to_nid.get(&svc.host_id) {
            edges.push(NetworkGraphEdge { edge_id, from_node: h_nid, to_node: svnid, edge_type: NetworkEdgeType::ServesOver, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
            edge_id += 1;
        }
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: svnid, edge_type: NetworkEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Cross-modal: service → code (101)
        edges.push(NetworkGraphEdge { edge_id, from_node: svnid, to_node: svnid, edge_type: NetworkEdgeType::ImplementedByCode, weight: 0.7, provenance: EdgeProvenance::DerivedFromCrossModal, version: 1, properties: { let mut p = HashMap::new(); p.insert("target_modality".into(), serde_json::json!("code")); p }, ..Default::default() });
        edge_id += 1;
        svc_id_to_nid.insert(svc.service_id, svnid);
        node_id += 1;
    }

    // ── SERVICE DEPENDENCY EDGES ──
    for dep in &analysis.service_dependencies {
        if let (Some(&up_nid), Some(&down_nid)) = (svc_id_to_nid.get(&dep.upstream_service_id), svc_id_to_nid.get(&dep.downstream_service_id)) {
            edges.push(NetworkGraphEdge {
                edge_id, from_node: down_nid, to_node: up_nid,
                edge_type: NetworkEdgeType::DependsOnService, weight: if dep.is_critical { 1.0 } else { 0.7 },
                provenance: EdgeProvenance::DerivedFromPrompt, version: 1,
                properties: {
                    let mut p = HashMap::new();
                    p.insert("is_critical".into(), serde_json::json!(dep.is_critical));
                    p.insert("dep_type".into(), serde_json::json!(format!("{:?}", dep.dependency_type)));
                    if let Some(lat) = dep.avg_latency_ms { p.insert("avg_latency_ms".into(), serde_json::json!(lat)); }
                    p
                },
                ..Default::default()
            });
            edge_id += 1;
        }
    }

    // ── BGP PEER NODES ──
    for peer in &analysis.bgp_peers {
        let pnid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: pnid, node_type: NetworkNodeType::HostNode,    // reuse HostNode for BGP peer
            content: format!("BGP Peer: {}/{} → {}/{} state={:?} rx_pfx={} tx_pfx={}",
                peer.local_ip, peer.local_as, peer.peer_ip, peer.peer_as,
                peer.state, peer.prefixes_received, peer.prefixes_sent),
            ip_address: Some(peer.peer_ip.clone()),
            asn: Some(peer.peer_as),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/BGPPeer/{}", project_id, graph_id, peer.peer_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["bgp".into(), "peer".into(), format!("AS{}", peer.peer_as)],
            hotness_score: if peer.state == BGPSessionState::Established { 0.75 } else { 0.5 },
            ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: pnid, edge_type: NetworkEdgeType::Contains, weight: 0.7, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Find local router and link BGPPeersWith
        for router in &analysis.routers {
            if let Some(bgp_as) = router.bgp_as {
                if bgp_as == peer.local_as {
                    if let Some(&router_host_nid) = host_id_to_nid.get(&router.host_id) {
                        edges.push(NetworkGraphEdge { edge_id, from_node: router_host_nid, to_node: pnid, edge_type: NetworkEdgeType::BGPPeersWith, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                        edge_id += 1;
                    }
                }
            }
        }
        node_id += 1;
    }

    // ── FLOW PATH NODES (sample top 50 flows) ──
    for flow in analysis.flow_paths.iter().take(50) {
        let fnid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: fnid, node_type: NetworkNodeType::FlowPathNode,
            content: format!("Flow: {}:{} → {}:{} {:?} bytes={} dir={:?} enc={}",
                flow.src_ip, flow.src_port.unwrap_or(0), flow.dst_ip, flow.dst_port.unwrap_or(0),
                flow.protocol, flow.bytes, flow.flow_direction, flow.is_encrypted),
            ip_address: Some(flow.src_ip.clone()),
            port: flow.dst_port,
            protocol: Some(format!("{:?}", flow.protocol)),
            bytes: Some(flow.bytes),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Flow/{}", project_id, graph_id, flow.flow_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["flow".into(), format!("{:?}", flow.protocol).to_lowercase(), format!("{:?}", flow.flow_direction).to_lowercase()],
            hotness_score: (flow.bytes as f32 / 1_000_000.0).min(0.95).max(0.4),
            ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: fnid, edge_type: NetworkEdgeType::Contains, weight: 0.6, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Src host → flow
        if let Some(src_host) = analysis.hosts.iter().find(|h| h.ip_addresses.contains(&flow.src_ip)) {
            if let Some(&sh_nid) = host_id_to_nid.get(&src_host.host_id) {
                edges.push(NetworkGraphEdge { edge_id, from_node: sh_nid, to_node: fnid, edge_type: NetworkEdgeType::TransmitsTo, weight: (flow.bytes as f32 / 1_000_000.0).min(1.0), provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── ANOMALY NODES ──
    for anomaly in &analysis.anomalies {
        let anid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: anid, node_type: NetworkNodeType::AnomalyNode,
            content: format!("Anomaly {:?} [{:?}]: {}", anomaly.anomaly_type, anomaly.severity, anomaly.description.chars().take(60).collect::<String>()),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/Anomaly/{}", project_id, graph_id, anomaly.anomaly_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["anomaly".into(), format!("{:?}", anomaly.anomaly_type).to_lowercase()],
            hotness_score: match anomaly.severity { AnomalySeverity::Critical => 0.98, AnomalySeverity::High => 0.88, AnomalySeverity::Medium => 0.75, _ => 0.55 },
            ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: anid, edge_type: NetworkEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1;
        // Anomaly → affected hosts
        for &affected_hid in &anomaly.affected_host_ids {
            if let Some(&h_nid) = host_id_to_nid.get(&affected_hid) {
                edges.push(NetworkGraphEdge { edge_id, from_node: anid, to_node: h_nid, edge_type: NetworkEdgeType::Affects, weight: 1.0, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
                edge_id += 1;
            }
        }
        node_id += 1;
    }

    // ── CLOUD VPC NODES ──
    for vpc in &analysis.cloud_vpcs {
        let vnid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: vnid, node_type: NetworkNodeType::CloudVPCNode,
            content: format!("CloudVPC [{}] {}: {} region={} subnets={} hosts={}",
                vpc.cloud_provider, vpc.vpc_identifier, vpc.cidr_blocks.first().unwrap_or(&"?".to_string()), vpc.region, vpc.subnet_count, vpc.host_count),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/VPC/{}", project_id, graph_id, vpc.vpc_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["cloud".into(), "vpc".into(), vpc.cloud_provider.to_lowercase(), vpc.region.to_lowercase()],
            hotness_score: 0.75, ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: vnid, edge_type: NetworkEdgeType::Contains, weight: 0.9, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── K8s SERVICE NODES ──
    for k8s_svc in &analysis.kubernetes_services {
        let knid = node_id;
        nodes.push(NetworkGraphNode {
            node_id: knid, node_type: NetworkNodeType::K8sServiceNode,
            content: format!("K8sService [{:?}] {}/{}: clusterIP={:?} pods={} endpoints={}",
                k8s_svc.service_type, k8s_svc.namespace, k8s_svc.name,
                k8s_svc.cluster_ip, k8s_svc.pod_count, k8s_svc.endpoint_count),
            ip_address: k8s_svc.cluster_ip.clone().or_else(|| k8s_svc.external_ip.clone()),
            service_name: Some(k8s_svc.name.clone()),
            materialized_path: Some(format!("/Modalities/NetworkTopology/Project_{}/Graph_{}/K8sSvc/{}", project_id, graph_id, k8s_svc.service_id)),
            provisional: false, provisional_status: ProvisionalStatus::Validated, version: 1,
            keywords: vec!["kubernetes".into(), "k8s".into(), k8s_svc.name.to_lowercase(), k8s_svc.namespace.to_lowercase()],
            hotness_score: if k8s_svc.service_type == K8sServiceType::LoadBalancer { 0.85 } else { 0.65 },
            ..Default::default()
        });
        edges.push(NetworkGraphEdge { edge_id, from_node: root_id, to_node: knid, edge_type: NetworkEdgeType::Contains, weight: 0.8, provenance: EdgeProvenance::DerivedFromPrompt, version: 1, ..Default::default() });
        edge_id += 1; node_id += 1;
    }

    // ── HOOK 1: OnGraphCreated ──
    let _ = executor.save_graph(&NetworkGraph { graph_id, project_id, source_description: analysis.source_description.clone(), nodes: nodes.clone(), edges: edges.clone(), root_node_id: root_id, state: GraphStateType::Created, state_history: vec![GraphStateTransition { from: GraphStateType::Created, to: GraphStateType::Created, timestamp: now.clone(), triggered_by_step: None }], created_at: now.clone(), updated_at: now.clone(), version: 1, version_notes: vec![VersionNote { version: 1, note: format!("Created: {} nodes {} edges", nodes.len(), edges.len()), step_index: None, timestamp: now.clone(), change_type: ChangeType::
