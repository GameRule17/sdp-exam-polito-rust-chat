/*
Modulo State: mantiene lo stato globale del server, inclusi utenti, gruppi, inviti e canali di comunicazione.
Fornisce strutture dati condivise tra i vari task.
*/

use ruggine_common::ServerToClient;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
use uuid::Uuid;

pub type Tx = mpsc::UnboundedSender<ServerToClient>;
pub type Rx = mpsc::UnboundedReceiver<ServerToClient>;

#[derive(Default)]
pub struct Group {
    pub members: HashSet<Uuid>, // ID dei client associati al gruppo
}

//users_by_nick e nicks_by_id vengono utilizzate entrambe per avere efficienza nelle ricerche
//altrimenti servirebbe un O(n) per scorrere nel caso opposto (per scorrere tutta la mappa)
#[derive(Default)]
pub struct State {
    pub users_by_nick: HashMap<String, Uuid>,
    // Mappa nickname -> UUID utente 
    //(associa ogni nickname registrato all'ID univoco del client)
    pub nicks_by_id: HashMap<Uuid, String>,
    // Mappa UUID utente -> nickname 
    //(associa ogni ID univoco al nickname corrispondente)
    pub groups: HashMap<String, Group>,
    // Mappa nome gruppo -> struttura Group 
    //(contiene tutti i gruppi attivi e i loro membri)
    pub invites: HashMap<String, (String, String)>,
    // Mappa codice invito -> (nome gruppo, nickname destinatario) 
    //(contiene tutti i codici invito attivi)
    pub clients: HashMap<Uuid, Tx>,
    // Mappa UUID utente -> canale di invio (Tx) 
    //(associa ogni client connesso al suo canale di comunicazione)
}
