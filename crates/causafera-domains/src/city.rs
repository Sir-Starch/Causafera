use crate::economy::EconomicState;
use causafera_types::{
    BuildingRecordId, EntityId, InfrastructureLinkId, InfrastructureNetworkId,
    InfrastructureNodeId, InfrastructureSchemaId, InventoryLotId, ParcelRecordId, PlaceId,
    SimulationTime, TraceId,
};

pub const MAX_PARCELS: usize = 32_768;
pub const MAX_BUILDINGS: usize = 32_768;
pub const MAX_INFRASTRUCTURE_NETWORKS: usize = 4_096;
pub const MAX_INFRASTRUCTURE_NODES: usize = 65_536;
pub const MAX_INFRASTRUCTURE_LINKS: usize = 65_536;
pub const MAX_MATERIAL_COMPONENTS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParcelRecord {
    pub id: ParcelRecordId,
    pub place: PlaceId,
    pub recorded_at: SimulationTime,
    pub trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingRecord {
    pub id: BuildingRecordId,
    pub entity: EntityId,
    pub parcel: ParcelRecordId,
    pub material_lots: Vec<InventoryLotId>,
    pub condition: u32,
    pub recorded_at: SimulationTime,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InfrastructureNetwork {
    pub id: InfrastructureNetworkId,
    pub schema: InfrastructureSchemaId,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InfrastructureNode {
    pub id: InfrastructureNodeId,
    pub network: InfrastructureNetworkId,
    pub place: PlaceId,
    pub capacity: u64,
    pub condition: u32,
    pub trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfrastructureLink {
    pub id: InfrastructureLinkId,
    pub network: InfrastructureNetworkId,
    pub source: InfrastructureNodeId,
    pub target: InfrastructureNodeId,
    pub capacity: u64,
    pub length_mm: u64,
    pub condition: u32,
    pub material_lots: Vec<InventoryLotId>,
    pub trace: TraceId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CityState {
    parcels: Vec<ParcelRecord>,
    buildings: Vec<BuildingRecord>,
    networks: Vec<InfrastructureNetwork>,
    nodes: Vec<InfrastructureNode>,
    links: Vec<InfrastructureLink>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CityError {
    CapacityExceeded,
    DuplicateId,
    DuplicateReference,
    SelfLink,
    UnknownParcel,
    UnknownNetwork,
    UnknownNode,
    UnknownMaterialLot,
    CrossNetworkLink,
    ZeroCapacity,
    ZeroLength,
}

impl CityState {
    pub fn new(
        mut parcels: Vec<ParcelRecord>,
        mut buildings: Vec<BuildingRecord>,
        mut networks: Vec<InfrastructureNetwork>,
        mut nodes: Vec<InfrastructureNode>,
        mut links: Vec<InfrastructureLink>,
        economy: &EconomicState,
    ) -> Result<Self, CityError> {
        if parcels.len() > MAX_PARCELS
            || buildings.len() > MAX_BUILDINGS
            || networks.len() > MAX_INFRASTRUCTURE_NETWORKS
            || nodes.len() > MAX_INFRASTRUCTURE_NODES
            || links.len() > MAX_INFRASTRUCTURE_LINKS
        {
            return Err(CityError::CapacityExceeded);
        }
        parcels.sort_unstable_by_key(|record| record.id);
        buildings.sort_unstable_by_key(|record| record.id);
        networks.sort_unstable_by_key(|record| record.id);
        nodes.sort_unstable_by_key(|record| record.id);
        links.sort_unstable_by_key(|record| record.id);
        reject_duplicates(&parcels, |record| record.id)?;
        reject_duplicates(&buildings, |record| record.id)?;
        reject_duplicates(&networks, |record| record.id)?;
        reject_duplicates(&nodes, |record| record.id)?;
        reject_duplicates(&links, |record| record.id)?;

        for building in &mut buildings {
            require_parcel(&parcels, building.parcel)?;
            validate_lots(&mut building.material_lots, economy)?;
        }
        for node in &nodes {
            require_network(&networks, node.network)?;
            if node.capacity == 0 {
                return Err(CityError::ZeroCapacity);
            }
        }
        for link in &mut links {
            if link.source == link.target {
                return Err(CityError::SelfLink);
            }
            if link.capacity == 0 {
                return Err(CityError::ZeroCapacity);
            }
            if link.length_mm == 0 {
                return Err(CityError::ZeroLength);
            }
            require_network(&networks, link.network)?;
            let source = find_node(&nodes, link.source)?;
            let target = find_node(&nodes, link.target)?;
            if source.network != link.network || target.network != link.network {
                return Err(CityError::CrossNetworkLink);
            }
            validate_lots(&mut link.material_lots, economy)?;
        }
        Ok(Self {
            parcels,
            buildings,
            networks,
            nodes,
            links,
        })
    }

    pub fn parcels(&self) -> &[ParcelRecord] {
        &self.parcels
    }
    pub fn buildings(&self) -> &[BuildingRecord] {
        &self.buildings
    }
    pub fn networks(&self) -> &[InfrastructureNetwork] {
        &self.networks
    }
    pub fn nodes(&self) -> &[InfrastructureNode] {
        &self.nodes
    }
    pub fn links(&self) -> &[InfrastructureLink] {
        &self.links
    }
    pub fn outgoing_links(
        &self,
        node: InfrastructureNodeId,
    ) -> impl Iterator<Item = &InfrastructureLink> {
        self.links.iter().filter(move |link| link.source == node)
    }
}

fn reject_duplicates<T, K: Eq + Copy>(
    values: &[T],
    key: impl Fn(&T) -> K,
) -> Result<(), CityError> {
    if values.windows(2).any(|pair| key(&pair[0]) == key(&pair[1])) {
        Err(CityError::DuplicateId)
    } else {
        Ok(())
    }
}
fn require_parcel(values: &[ParcelRecord], id: ParcelRecordId) -> Result<(), CityError> {
    values
        .binary_search_by_key(&id, |record| record.id)
        .map(|_| ())
        .map_err(|_| CityError::UnknownParcel)
}
fn require_network(
    values: &[InfrastructureNetwork],
    id: InfrastructureNetworkId,
) -> Result<(), CityError> {
    values
        .binary_search_by_key(&id, |record| record.id)
        .map(|_| ())
        .map_err(|_| CityError::UnknownNetwork)
}
fn find_node(
    values: &[InfrastructureNode],
    id: InfrastructureNodeId,
) -> Result<&InfrastructureNode, CityError> {
    values
        .binary_search_by_key(&id, |record| record.id)
        .map(|index| &values[index])
        .map_err(|_| CityError::UnknownNode)
}
fn validate_lots(values: &mut [InventoryLotId], economy: &EconomicState) -> Result<(), CityError> {
    if values.len() > MAX_MATERIAL_COMPONENTS {
        return Err(CityError::CapacityExceeded);
    }
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CityError::DuplicateReference);
    }
    if values.iter().any(|id| economy.lot(*id).is_none()) {
        return Err(CityError::UnknownMaterialLot);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::InventoryLot;
    use causafera_types::{MaterialId, PropertyClaimId};

    fn economy() -> EconomicState {
        EconomicState::new(
            vec![InventoryLot {
                id: InventoryLotId::new(1),
                material: MaterialId::new(2),
                holder: EntityId::new(3),
                location: PlaceId::new(4),
                quantity: 5,
                ownership_claims: vec![PropertyClaimId::new(9)],
                recorded_at: SimulationTime::new(1),
                trace: TraceId::new(1),
            }],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
    }
    fn network() -> InfrastructureNetwork {
        InfrastructureNetwork {
            id: InfrastructureNetworkId::new(1),
            schema: InfrastructureSchemaId::new(99),
            trace: TraceId::new(2),
        }
    }
    fn nodes() -> Vec<InfrastructureNode> {
        vec![
            InfrastructureNode {
                id: InfrastructureNodeId::new(2),
                network: InfrastructureNetworkId::new(1),
                place: PlaceId::new(2),
                capacity: 4,
                condition: 7,
                trace: TraceId::new(3),
            },
            InfrastructureNode {
                id: InfrastructureNodeId::new(1),
                network: InfrastructureNetworkId::new(1),
                place: PlaceId::new(1),
                capacity: 4,
                condition: 7,
                trace: TraceId::new(4),
            },
        ]
    }

    #[test]
    fn city_state_is_canonical_and_network_semantics_are_opaque() {
        let state = CityState::new(
            vec![ParcelRecord {
                id: ParcelRecordId::new(1),
                place: PlaceId::new(10),
                recorded_at: SimulationTime::new(1),
                trace: TraceId::new(1),
            }],
            vec![BuildingRecord {
                id: BuildingRecordId::new(1),
                entity: EntityId::new(8),
                parcel: ParcelRecordId::new(1),
                material_lots: vec![InventoryLotId::new(1)],
                condition: 9,
                recorded_at: SimulationTime::new(2),
                trace: TraceId::new(2),
            }],
            vec![network()],
            nodes(),
            vec![InfrastructureLink {
                id: InfrastructureLinkId::new(1),
                network: InfrastructureNetworkId::new(1),
                source: InfrastructureNodeId::new(1),
                target: InfrastructureNodeId::new(2),
                capacity: 3,
                length_mm: 20,
                condition: 8,
                material_lots: vec![InventoryLotId::new(1)],
                trace: TraceId::new(5),
            }],
            &economy(),
        )
        .unwrap();
        assert_eq!(state.nodes()[0].id, InfrastructureNodeId::new(1));
        assert_eq!(
            state.outgoing_links(InfrastructureNodeId::new(1)).count(),
            1
        );
        assert_eq!(state.networks()[0].schema, InfrastructureSchemaId::new(99));
    }

    #[test]
    fn cross_network_and_unknown_material_references_are_rejected() {
        let mut other_nodes = nodes();
        other_nodes[0].network = InfrastructureNetworkId::new(2);
        let link = InfrastructureLink {
            id: InfrastructureLinkId::new(1),
            network: InfrastructureNetworkId::new(1),
            source: InfrastructureNodeId::new(1),
            target: InfrastructureNodeId::new(2),
            capacity: 3,
            length_mm: 20,
            condition: 8,
            material_lots: vec![],
            trace: TraceId::new(5),
        };
        assert_eq!(
            CityState::new(
                vec![],
                vec![],
                vec![
                    network(),
                    InfrastructureNetwork {
                        id: InfrastructureNetworkId::new(2),
                        schema: InfrastructureSchemaId::new(4),
                        trace: TraceId::new(6)
                    }
                ],
                other_nodes,
                vec![link],
                &economy()
            ),
            Err(CityError::CrossNetworkLink)
        );
    }
}
