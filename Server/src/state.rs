/*
Modulo State: mantiene lo stato globale del server, inclusi utenti, gruppi, inviti e canali di comunicazione.
Fornisce strutture dati condivise tra i vari task.
*/

use std::collections::{HashMap, HashSet};
use ruggine_common::ServerToClient;
use tokio::sync::mpsc;
use uuid::Uuid;

pub type Tx = mpsc::UnboundedSender<ServerToClient>;
pub type Rx = mpsc::UnboundedReceiver<ServerToClient>;

#[derive(Default)]
pub struct Group {
    pub members: HashSet<Uuid>, // ID dei client
}

#[derive(Default)]
pub struct State {
    pub users_by_nick: HashMap<String, Uuid>,
    pub nicks_by_id: HashMap<Uuid, String>,
    pub groups: HashMap<String, Group>,
    pub invites: HashMap<String, (String, String)>,
    pub clients: HashMap<Uuid, Tx>,
}
