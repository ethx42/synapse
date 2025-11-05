use crate::client::constants::OSI_ANIMATION_SAMPLE_RATE;
use colored::*;

/// Represents the position of a packet in the OSI model visualization
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PacketPosition {
    ClientL7,
    ClientL4,
    ClientL3,
    ClientL2,
    ClientL1,
    ServerL1,
    ServerL2,
    ServerL3,
    ServerL4,
    ServerL7,
    ReturnServerL7,
    ReturnServerL4,
    ReturnServerL3,
    ReturnServerL2,
    ReturnServerL1,
    ReturnClientL1,
    ReturnClientL2,
    ReturnClientL3,
    ReturnClientL4,
    ReturnClientL7,
}

impl PacketPosition {
    fn next(self) -> Self {
        match self {
            PacketPosition::ClientL7 => PacketPosition::ClientL4,
            PacketPosition::ClientL4 => PacketPosition::ClientL3,
            PacketPosition::ClientL3 => PacketPosition::ClientL2,
            PacketPosition::ClientL2 => PacketPosition::ClientL1,
            PacketPosition::ClientL1 => PacketPosition::ServerL1,
            PacketPosition::ServerL1 => PacketPosition::ServerL2,
            PacketPosition::ServerL2 => PacketPosition::ServerL3,
            PacketPosition::ServerL3 => PacketPosition::ServerL4,
            PacketPosition::ServerL4 => PacketPosition::ServerL7,
            PacketPosition::ServerL7 => PacketPosition::ReturnServerL7,
            PacketPosition::ReturnServerL7 => PacketPosition::ReturnServerL4,
            PacketPosition::ReturnServerL4 => PacketPosition::ReturnServerL3,
            PacketPosition::ReturnServerL3 => PacketPosition::ReturnServerL2,
            PacketPosition::ReturnServerL2 => PacketPosition::ReturnServerL1,
            PacketPosition::ReturnServerL1 => PacketPosition::ReturnClientL1,
            PacketPosition::ReturnClientL1 => PacketPosition::ReturnClientL2,
            PacketPosition::ReturnClientL2 => PacketPosition::ReturnClientL3,
            PacketPosition::ReturnClientL3 => PacketPosition::ReturnClientL4,
            PacketPosition::ReturnClientL4 => PacketPosition::ReturnClientL7,
            PacketPosition::ReturnClientL7 => PacketPosition::ClientL7,
        }
    }
}

struct OsiState {
    position: PacketPosition,
}

impl OsiState {
    fn new() -> Self {
        Self {
            position: PacketPosition::ClientL7,
        }
    }

    fn advance(&mut self) {
        self.position = self.position.next();
    }
}

fn render_layer(label: &str, detail: &str, is_active: bool, layer_color: (u8, u8, u8)) -> String {
    let (r, g, b) = layer_color;
    let text = format!("{}: {}", label, detail);

    if is_active {
        // Bright background color when active with white text
        format!(
            "{}",
            format!(" {:<20} ", text)
                .on_truecolor(r, g, b)
                .truecolor(255, 255, 255)
                .bold()
        )
    } else {
        // Dim background color when inactive (reduce brightness by ~70%)
        let dim_r = (r as f32 * 0.3) as u8;
        let dim_g = (g as f32 * 0.3) as u8;
        let dim_b = (b as f32 * 0.3) as u8;
        format!(
            "{}",
            format!(" {:<20} ", text)
                .on_truecolor(dim_r, dim_g, dim_b)
                .truecolor(100, 100, 100)
        )
    }
}

fn render_osi_stack(osi_state: &OsiState) -> String {
    let pos = osi_state.position;

    // Check which layers are active on each side
    let client_l7_active = matches!(
        pos,
        PacketPosition::ClientL7 | PacketPosition::ReturnClientL7
    );
    let client_l4_active = matches!(
        pos,
        PacketPosition::ClientL4 | PacketPosition::ReturnClientL4
    );
    let client_l3_active = matches!(
        pos,
        PacketPosition::ClientL3 | PacketPosition::ReturnClientL3
    );
    let client_l2_active = matches!(
        pos,
        PacketPosition::ClientL2 | PacketPosition::ReturnClientL2
    );
    let client_l1_active = matches!(
        pos,
        PacketPosition::ClientL1 | PacketPosition::ReturnClientL1
    );

    let server_l7_active = matches!(
        pos,
        PacketPosition::ServerL7 | PacketPosition::ReturnServerL7
    );
    let server_l4_active = matches!(
        pos,
        PacketPosition::ServerL4 | PacketPosition::ReturnServerL4
    );
    let server_l3_active = matches!(
        pos,
        PacketPosition::ServerL3 | PacketPosition::ReturnServerL3
    );
    let server_l2_active = matches!(
        pos,
        PacketPosition::ServerL2 | PacketPosition::ReturnServerL2
    );
    let server_l1_active = matches!(
        pos,
        PacketPosition::ServerL1 | PacketPosition::ReturnServerL1
    );

    // Layer colors (RGB): Blue, Green, Yellow, Orange, Red
    let l7_color = (74, 144, 226); // Blue
    let l4_color = (72, 187, 120); // Green
    let l3_color = (236, 201, 75); // Yellow
    let l2_color = (237, 137, 54); // Orange
    let l1_color = (245, 101, 101); // Red

    let mut lines = Vec::new();

    // Header - centered above stacks
    lines.push(format!(
        "          {}                  {}",
        "CLIENT".bold(),
        "SERVER".bold()
    ));

    // Layer 7
    lines.push(format!(
        "{}  {}",
        render_layer("L7", "APPLICATION", client_l7_active, l7_color),
        render_layer("L7", "APPLICATION", server_l7_active, l7_color)
    ));

    // Layer 4
    lines.push(format!(
        "{}  {}",
        render_layer("L4", "TRANSPORT", client_l4_active, l4_color),
        render_layer("L4", "TRANSPORT", server_l4_active, l4_color)
    ));

    // Layer 3
    lines.push(format!(
        "{}  {}",
        render_layer("L3", "NETWORK", client_l3_active, l3_color),
        render_layer("L3", "NETWORK", server_l3_active, l3_color)
    ));

    // Layer 2
    lines.push(format!(
        "{}  {}",
        render_layer("L2", "DATA LINK", client_l2_active, l2_color),
        render_layer("L2", "DATA LINK", server_l2_active, l2_color)
    ));

    // Layer 1
    lines.push(format!(
        "{}  {}",
        render_layer("L1", "PHYSICAL", client_l1_active, l1_color),
        render_layer("L1", "PHYSICAL", server_l1_active, l1_color)
    ));

    // Break line
    lines.push("                      ".to_string());
    lines.join("\n")
}

/// OSI layer visualization manager
pub struct OsiVisualizer {
    state: OsiState,
    sample_rate: usize,
}

impl OsiVisualizer {
    /// Create a new OSI visualizer
    pub fn new() -> Self {
        Self {
            state: OsiState::new(),
            sample_rate: OSI_ANIMATION_SAMPLE_RATE,
        }
    }

    /// Check if the visualizer should update based on packet index
    pub fn should_update(&self, packet_index: usize) -> bool {
        (packet_index + 1) % self.sample_rate == 0
    }

    /// Advance the visualization state
    pub fn advance(&mut self) {
        self.state.advance();
    }

    /// Render the OSI stack visualization
    pub fn render(&self) -> String {
        render_osi_stack(&self.state)
    }

    /// Get the current packet position (for testing/debugging)
    #[cfg(test)]
    fn current_position(&self) -> PacketPosition {
        self.state.position
    }
}

impl Default for OsiVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualizer_new() {
        let viz = OsiVisualizer::new();
        assert!(!viz.render().is_empty());
    }

    #[test]
    fn test_visualizer_should_update() {
        let viz = OsiVisualizer::new();
        // Should update every 100th packet (OSI_ANIMATION_SAMPLE_RATE)
        assert!(viz.should_update(99)); // packet_index 99 = packet 100
        assert!(!viz.should_update(98));
        assert!(viz.should_update(199)); // packet_index 199 = packet 200
    }

    #[test]
    fn test_visualizer_advance() {
        let mut viz = OsiVisualizer::new();
        assert_eq!(viz.current_position(), PacketPosition::ClientL7);
        let initial_render = viz.render();

        // Advance through enough positions to see a clear difference
        // From ClientL7 -> ClientL4 -> ClientL3 -> ClientL2 -> ClientL1 -> ServerL1
        // This should change from Client L7 active to Server L1 active
        for _ in 0..5 {
            viz.advance();
        }
        assert_eq!(viz.current_position(), PacketPosition::ServerL1);
        let after_render = viz.render();
        // After advancing through different layers, the visualization should change
        // The active layer should move from Client L7 to Server L1
        assert_ne!(
            initial_render, after_render,
            "Visualization should change after advancing positions"
        );
    }

    #[test]
    fn test_visualizer_default() {
        let viz1 = OsiVisualizer::new();
        let viz2 = OsiVisualizer::default();
        assert_eq!(viz1.render(), viz2.render());
    }

    #[test]
    fn test_packet_position_cycle() {
        let mut pos = PacketPosition::ClientL7;
        let start = pos;

        // Count positions: There are 20 positions in the cycle
        // ClientL7 -> ClientL4 -> ClientL3 -> ClientL2 -> ClientL1 ->
        // ServerL1 -> ServerL2 -> ServerL3 -> ServerL4 -> ServerL7 ->
        // ReturnServerL7 -> ReturnServerL4 -> ReturnServerL3 -> ReturnServerL2 -> ReturnServerL1 ->
        // ReturnClientL1 -> ReturnClientL2 -> ReturnClientL3 -> ReturnClientL4 -> ReturnClientL7 -> ClientL7

        // Advance through all positions in the cycle
        for _ in 0..20 {
            pos = pos.next();
        }

        // Should cycle back to start
        assert_eq!(pos, start);
    }
}
