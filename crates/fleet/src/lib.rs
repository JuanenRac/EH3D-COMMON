//! Several machines and stations at once. The EH3D server keeps every machine, in the station it belongs to, and answers for all of them in
//! one request (`GET /api/v1/fleet`); this crate reads that answer into types that a central screen can use: which machines there are,
//! in which stations, what each one is doing and how far its job has got, which one is free for the next job. It does not talk to the
//! network: a program makes the request in its own way and gives the JSON to [`Fleet::from_json`].

use serde_json::Value;

/// What a machine is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineState {
    /// No job is running on it.
    Idle,
    /// A job is running.
    Busy,
    /// A job is held at a question for the operator (the tool change, the spindle).
    Waiting,
}

impl MachineState {
    fn from_word(w: &str) -> MachineState {
        match w {
            "busy" => MachineState::Busy,
            "waiting" => MachineState::Waiting,
            _ => MachineState::Idle,
        }
    }

    /// The word of the server for it.
    pub fn word(self) -> &'static str {
        match self {
            MachineState::Idle => "idle",
            MachineState::Busy => "busy",
            MachineState::Waiting => "waiting",
        }
    }
}

/// The job that is running on a machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub id: String,
    pub name: String,
    /// `drill` or `route`.
    pub kind: String,
    /// How far it has got, 0 to 100.
    pub percent: u8,
    pub items_done: usize,
    pub items_total: usize,
    /// Who asked for it.
    pub by: String,
    /// The question it waits at (`tool_change` or `spindle_start`), if any.
    pub question: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub model: String,
    /// The station it belongs to; empty when it has none.
    pub station: String,
    /// `demo` (the simulator) or `node`.
    pub connection: String,
    pub state: MachineState,
    pub job: Option<Job>,
    /// How many jobs are ready on it, waiting for an operator to start them.
    pub ready: usize,
}

/// The machines of one station (the station without a name holds the machines that have none).
#[derive(Debug, Clone, PartialEq)]
pub struct Station {
    pub id: String,
    pub machines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fleet {
    pub stations: Vec<Station>,
    pub machines: Vec<Machine>,
}

/// The counts of the fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    pub machines: usize,
    pub idle: usize,
    pub busy: usize,
    pub waiting: usize,
    /// The stations that have a name.
    pub stations: usize,
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

fn count(v: &Value, key: &str) -> usize {
    v.get(key).and_then(Value::as_u64).unwrap_or(0) as usize
}

impl Fleet {
    /// Reads the answer of the server. A machine without an `id` is an error (nothing could be done with it); the rest that is missing
    /// takes its empty value.
    pub fn from_json(v: &Value) -> Result<Fleet, String> {
        let machines_json = v
            .get("machines")
            .and_then(Value::as_array)
            .ok_or("the answer has no list of machines")?;
        let mut machines = Vec::with_capacity(machines_json.len());
        for m in machines_json {
            let id = text(m, "id");
            if id.is_empty() {
                return Err("a machine has no id".to_string());
            }
            let job = m.get("job").filter(|j| j.is_object()).map(|j| Job {
                id: text(j, "id"),
                name: text(j, "name"),
                kind: text(j, "kind"),
                percent: count(j, "percent").min(100) as u8,
                items_done: count(j, "items_done"),
                items_total: count(j, "items_total"),
                by: text(j, "by"),
                question: j
                    .get("pause")
                    .filter(|p| p.is_object())
                    .map(|p| text(p, "kind")),
            });
            machines.push(Machine {
                name: if text(m, "name").is_empty() {
                    id.clone()
                } else {
                    text(m, "name")
                },
                id,
                model: text(m, "model"),
                station: text(m, "station"),
                connection: text(m, "connection"),
                state: MachineState::from_word(&text(m, "state")),
                job,
                ready: count(m, "ready"),
            });
        }
        // the stations as the server grouped them; a machine the list leaves out is put in the station it says
        let mut stations: Vec<Station> = v
            .get("stations")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|s| Station {
                        id: text(s, "id"),
                        machines: s
                            .get("machines")
                            .and_then(Value::as_array)
                            .map(|l| {
                                l.iter()
                                    .filter_map(|x| x.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        for m in &machines {
            if !stations.iter().any(|s| s.machines.contains(&m.id)) {
                match stations.iter_mut().find(|s| s.id == m.station) {
                    Some(s) => s.machines.push(m.id.clone()),
                    None => stations.push(Station {
                        id: m.station.clone(),
                        machines: vec![m.id.clone()],
                    }),
                }
            }
        }
        Ok(Fleet { stations, machines })
    }

    pub fn find(&self, id: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.id == id)
    }

    /// The machines of a station, in the order of the server.
    pub fn machines_of(&self, station: &str) -> Vec<&Machine> {
        self.stations
            .iter()
            .find(|s| s.id == station)
            .map(|s| s.machines.iter().filter_map(|id| self.find(id)).collect())
            .unwrap_or_default()
    }

    /// The machines that no job is running on.
    pub fn free(&self) -> Vec<&Machine> {
        self.machines
            .iter()
            .filter(|m| m.state == MachineState::Idle)
            .collect()
    }

    /// The free machine with the fewest jobs ready, in a station if one is named (an empty `station` means any): where the next job
    /// should go. The first one of the server's order wins a tie.
    pub fn pick_free(&self, station: &str) -> Option<&Machine> {
        self.free()
            .into_iter()
            .filter(|m| station.is_empty() || m.station == station)
            .min_by_key(|m| m.ready)
    }

    pub fn summary(&self) -> Summary {
        let mut s = Summary {
            machines: self.machines.len(),
            ..Summary::default()
        };
        for m in &self.machines {
            match m.state {
                MachineState::Idle => s.idle += 1,
                MachineState::Busy => s.busy += 1,
                MachineState::Waiting => s.waiting += 1,
            }
        }
        s.stations = self.stations.iter().filter(|st| !st.id.is_empty()).count();
        s
    }

    /// The fleet as plain text, one line a machine under the heading of its station: for a terminal.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for st in &self.stations {
            out.push_str(&format!(
                "[{}]\n",
                if st.id.is_empty() { "-" } else { &st.id }
            ));
            for m in self.machines_of(&st.id) {
                let job = match &m.job {
                    Some(j) => format!(
                        "{} {}% ({}/{})",
                        j.name, j.percent, j.items_done, j.items_total
                    ),
                    None => String::new(),
                };
                out.push_str(&format!(
                    "  {:<14} {:<8} {:<7} ready {:<2} {}\n",
                    m.id,
                    m.state.word(),
                    m.connection,
                    m.ready,
                    job
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "stations": [
                {"id": "bench", "machines": ["bench-1", "bench-2"]},
                {"id": "room", "machines": ["room-1"]},
                {"id": "", "machines": ["spare"]}
            ],
            "machines": [
                {"id": "bench-1", "name": "Bench one", "model": "ccd-mtc", "station": "bench", "connection": "node", "state": "waiting",
                 "job": {"id": "j1", "name": "board", "kind": "drill", "state": "waiting_operator", "by": "juan", "items_done": 3, "items_total": 12,
                         "percent": 25, "pause": {"kind": "tool_change", "tool": 2}}, "ready": 1},
                {"id": "bench-2", "name": "bench-2", "station": "bench", "connection": "demo", "state": "idle", "job": null, "ready": 3},
                {"id": "room-1", "name": "Room", "station": "room", "connection": "demo", "state": "busy",
                 "job": {"id": "j2", "name": "frame", "kind": "route", "items_done": 5, "items_total": 10, "percent": 50, "pause": null}, "ready": 0},
                {"id": "spare", "name": "Spare", "station": "", "connection": "demo", "state": "idle", "job": null, "ready": 0}
            ],
            "summary": {"machines": 4, "idle": 2, "busy": 1, "waiting": 1, "stations": 2}
        })
    }

    #[test]
    fn the_answer_of_the_server_is_read_with_its_jobs_and_questions() {
        let f = Fleet::from_json(&sample()).unwrap();
        assert_eq!(f.machines.len(), 4);
        let b1 = f.find("bench-1").unwrap();
        assert_eq!(
            (b1.state, b1.name.as_str(), b1.ready),
            (MachineState::Waiting, "Bench one", 1)
        );
        let j = b1.job.as_ref().unwrap();
        assert_eq!((j.percent, j.items_done, j.items_total), (25, 3, 12));
        assert_eq!(j.question.as_deref(), Some("tool_change"));
        // a job that is not at a question has none
        assert_eq!(
            f.find("room-1").unwrap().job.as_ref().unwrap().question,
            None
        );
        assert!(f.find("bench-2").unwrap().job.is_none());
        assert!(f.find("none").is_none());
    }

    #[test]
    fn the_stations_group_the_machines_and_the_counts_agree_with_the_server() {
        let f = Fleet::from_json(&sample()).unwrap();
        let ids: Vec<&str> = f
            .machines_of("bench")
            .iter()
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(ids, vec!["bench-1", "bench-2"]);
        assert_eq!(f.machines_of("nowhere").len(), 0);
        let s = f.summary();
        assert_eq!(
            (s.machines, s.idle, s.busy, s.waiting, s.stations),
            (4, 2, 1, 1, 2)
        );
        assert_eq!(f.summary(), Fleet::from_json(&sample()).unwrap().summary());
    }

    #[test]
    fn the_next_job_goes_to_the_free_machine_with_the_fewest_jobs_ready() {
        let f = Fleet::from_json(&sample()).unwrap();
        // free: bench-2 (3 ready) and spare (0 ready)
        assert_eq!(f.pick_free("").unwrap().id, "spare");
        assert_eq!(f.pick_free("bench").unwrap().id, "bench-2");
        // no free machine in the room: its one is busy
        assert!(f.pick_free("room").is_none());
        assert_eq!(f.free().len(), 2);
    }

    #[test]
    fn a_machine_the_list_of_stations_leaves_out_is_put_in_its_station_and_a_bad_answer_is_refused()
    {
        let mut v = sample();
        v["stations"] = json!([{"id": "bench", "machines": ["bench-1"]}]);
        let f = Fleet::from_json(&v).unwrap();
        assert_eq!(f.machines_of("bench").len(), 2);
        assert_eq!(f.machines_of("room").len(), 1);
        assert!(Fleet::from_json(&json!({"stations": []})).is_err());
        assert!(Fleet::from_json(&json!({"machines": [{"name": "no id"}]})).is_err());
        // an empty fleet is a fleet
        assert_eq!(
            Fleet::from_json(&json!({"machines": []})).unwrap(),
            Fleet::default()
        );
    }

    #[test]
    fn the_text_lists_a_station_with_its_machines_and_their_jobs() {
        let t = Fleet::from_json(&sample()).unwrap().to_text();
        assert!(t.contains("[bench]") && t.contains("[room]") && t.contains("[-]"));
        assert!(t.contains("board 25% (3/12)"), "{t}");
        assert!(t
            .lines()
            .any(|l| l.contains("bench-2") && l.contains("idle")));
    }
}
