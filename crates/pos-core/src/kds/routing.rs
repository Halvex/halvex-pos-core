use pos_types::MenuItemID;

use super::StationId;

#[derive(Clone, Debug)]
pub struct RoutingRule {
    pub station: StationId,
    pub menu_item_id: Option<MenuItemID>,
    pub name_prefix: Option<String>,
}

impl RoutingRule {
    pub fn for_menu_item(menu_item_id: MenuItemID, station: impl Into<StationId>) -> Self {
        Self {
            station: station.into(),
            menu_item_id: Some(menu_item_id),
            name_prefix: None,
        }
    }

    pub fn for_name_prefix(prefix: impl Into<String>, station: impl Into<StationId>) -> Self {
        Self {
            station: station.into(),
            menu_item_id: None,
            name_prefix: Some(prefix.into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RoutingTable {
    pub rules: Vec<RoutingRule>,
    pub default_station: StationId,
}

impl RoutingTable {
    pub fn new(rules: Vec<RoutingRule>, default_station: impl Into<StationId>) -> Self {
        Self {
            rules,
            default_station: default_station.into(),
        }
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            default_station: "kitchen".to_string(),
        }
    }
}

pub fn station_for_item(
    menu_item_id: MenuItemID,
    name: &str,
    routing_table: &RoutingTable,
) -> StationId {
    for rule in &routing_table.rules {
        if let Some(rule_menu_item_id) = rule.menu_item_id {
            if rule_menu_item_id == menu_item_id {
                return rule.station.clone();
            }
        }

        if let Some(prefix) = &rule.name_prefix {
            if name.starts_with(prefix) {
                return rule.station.clone();
            }
        }
    }

    routing_table.default_station.clone()
}
