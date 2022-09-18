#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::wildcard_imports,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    unused,
    missing_docs
)]
#![allow(
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::multiple_crate_versions,
    clippy::needless_pass_by_value
)]

//! Provides a plugin for the Bevy game engine for renet networking diagnostics.

use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    ecs::{schedule::ShouldRun, system::Resource},
    prelude::*,
};
use bevy_renet::renet::{RenetClient, RenetServer};
use std::collections::HashSet;

/// Renet diagnostics plugin.
#[derive(Default)]
pub struct RenetDiagnosticsPlugin;

impl Plugin for RenetDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(
            Self::setup_client_system.with_run_criteria(has_resource::<RenetClient>),
        )
        .add_system(Self::diagnostic_client_system.with_run_criteria(has_resource::<RenetClient>))
        .add_system(Self::diagnostic_server_system.with_run_criteria(has_resource::<RenetServer>))
        .insert_resource(RenetDiagnosticsState::default());
    }
}

const BASE_RTT_ID: u128 = 41_598_154_451_286_296_937_086_289_020_957_009_281;
const BASE_RTT_NAME: &str = "network_rtt";
const BASE_SENT_KBPS_ID: u128 = 90_848_304_625_986_116_817_112_626_213_519_895_727;
const BASE_SENT_KBPS_NAME: &str = "network_sent_kbps";
const BASE_RECEIVED_KBPS_ID: u128 = 31_519_729_826_609_147_865_210_468_700_944_359_742;
const BASE_RECEIVED_KBPS_NAME: &str = "network_received_kbps";
const BASE_PACKET_LOSS_ID: u128 = 74_946_797_489_629_323_601_504_027_060_352_742_281;
const BASE_PACKET_LOSS_NAME: &str = "network_packet_loss";

impl RenetDiagnosticsPlugin {
    /// Diagnostics ID for roundtrip time (RTT).
    pub const RTT: DiagnosticId = DiagnosticId::from_u128(BASE_RTT_ID);
    /// Diagnostics ID for sent Kbps.
    pub const SENT_KBPS: DiagnosticId = DiagnosticId::from_u128(BASE_SENT_KBPS_ID);
    /// Diagnostics ID for received Kbps.
    pub const RECEIVED_KBPS: DiagnosticId = DiagnosticId::from_u128(BASE_RECEIVED_KBPS_ID);
    /// Diagnostics ID for packet loss.
    pub const PACKET_LOSS: DiagnosticId = DiagnosticId::from_u128(BASE_PACKET_LOSS_ID);

    fn setup_client_system(mut diagnostics: ResMut<'_, Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::RTT, BASE_RTT_NAME, 20));
        diagnostics.add(Diagnostic::new(Self::SENT_KBPS, BASE_SENT_KBPS_NAME, 20));
        diagnostics.add(Diagnostic::new(
            Self::RECEIVED_KBPS,
            BASE_RECEIVED_KBPS_NAME,
            20,
        ));
        diagnostics.add(Diagnostic::new(
            Self::PACKET_LOSS,
            BASE_PACKET_LOSS_NAME,
            20,
        ));
    }

    fn diagnostic_client_system(
        mut diagnostics: ResMut<'_, Diagnostics>,
        client: Res<'_, RenetClient>,
    ) {
        let info = client.network_info();
        diagnostics.add_measurement(Self::RTT, || info.rtt.into());
        diagnostics.add_measurement(Self::SENT_KBPS, || info.sent_kbps.into());
        diagnostics.add_measurement(Self::RECEIVED_KBPS, || info.received_kbps.into());
        diagnostics.add_measurement(Self::PACKET_LOSS, || info.packet_loss.into());
    }

    fn diagnostic_server_system(
        mut state: ResMut<'_, RenetDiagnosticsState>,
        mut diagnostics: ResMut<'_, Diagnostics>,
        server: Res<'_, RenetServer>,
    ) {
        for client_id in server.clients_id() {
            if let Some(info) = server.network_info(client_id) {
                state.track_client_id(diagnostics.as_mut(), client_id);
                diagnostics.add_measurement(client_rtt_id(client_id), || info.rtt.into());
                diagnostics
                    .add_measurement(client_sent_kbps_id(client_id), || info.sent_kbps.into());
                diagnostics.add_measurement(client_received_kbps_id(client_id), || {
                    info.received_kbps.into()
                });
                diagnostics
                    .add_measurement(client_packet_loss_id(client_id), || info.packet_loss.into());
            }
        }
    }
}

#[derive(Default)]
struct RenetDiagnosticsState {
    added_client_ids: HashSet<u64>,
}

impl RenetDiagnosticsState {
    fn track_client_id(&mut self, diagnostics: &mut Diagnostics, client_id: u64) {
        if self.added_client_ids.contains(&client_id) {
            return;
        }
        diagnostics.add(client_rtt(client_id));
        diagnostics.add(client_sent_kbps(client_id));
        diagnostics.add(client_received_kbps(client_id));
        diagnostics.add(client_packet_loss(client_id));
        self.added_client_ids.insert(client_id);
    }
}

fn client_rtt_id(client_id: u64) -> DiagnosticId {
    client_diagnostics_id(BASE_RTT_ID, client_id)
}

fn client_sent_kbps_id(client_id: u64) -> DiagnosticId {
    client_diagnostics_id(BASE_SENT_KBPS_ID, client_id)
}

fn client_received_kbps_id(client_id: u64) -> DiagnosticId {
    client_diagnostics_id(BASE_RECEIVED_KBPS_ID, client_id)
}

fn client_packet_loss_id(client_id: u64) -> DiagnosticId {
    client_diagnostics_id(BASE_PACKET_LOSS_ID, client_id)
}

fn client_diagnostics_id(base: u128, client_id: u64) -> DiagnosticId {
    DiagnosticId::from_u128(base + u128::from(client_id))
}

fn client_rtt(client_id: u64) -> Diagnostic {
    Diagnostic::new(
        client_rtt_id(client_id),
        client_diagnostics_name(BASE_RTT_NAME, client_id),
        20,
    )
}

fn client_sent_kbps(client_id: u64) -> Diagnostic {
    Diagnostic::new(
        client_sent_kbps_id(client_id),
        client_diagnostics_name(BASE_SENT_KBPS_NAME, client_id),
        20,
    )
}

fn client_received_kbps(client_id: u64) -> Diagnostic {
    Diagnostic::new(
        client_received_kbps_id(client_id),
        client_diagnostics_name(BASE_RECEIVED_KBPS_NAME, client_id),
        20,
    )
}

fn client_packet_loss(client_id: u64) -> Diagnostic {
    Diagnostic::new(
        client_packet_loss_id(client_id),
        client_diagnostics_name(BASE_PACKET_LOSS_NAME, client_id),
        20,
    )
}

fn client_diagnostics_name(base: &str, client_id: u64) -> String {
    format!("{}_{}", base, client_id)
}

fn has_resource<T: Resource>(resource: Option<Res<'_, T>>) -> ShouldRun {
    if resource.is_some() {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}
