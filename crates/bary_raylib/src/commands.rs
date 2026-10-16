use bary_core::prelude::*;
use bary_orbital::TerrainMaterial;
use bary_parts::BlueprintId;
use bary_sim::*;
use bary_terminal::*;

fn parse_arg<T>(args: &ArgsMap, key: &'static str) -> Result<T, ParseError>
where
    T: std::str::FromStr,
{
    let arg = args.get(key).ok_or(ParseError::BadKey)?;
    arg.parse().map_err(|_| ParseError::BadValue)
}

pub fn cmd_find(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let name = parse_arg(args, "grid_name")?;
    Ok(TermCmd::FindGridByName(name))
}

pub fn cmd_set_speed(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let speed = parse_arg(args, "speed")?;
    Ok(TermCmd::SetSimSpeed(speed))
}

pub fn cmd_placeholder(_args: &ArgsMap) -> Result<TermCmd, ParseError> {
    Err(ParseError::NotImplemented)
}

pub fn cmd_say(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let state = parse_arg(args, "msg")?;
    Ok(TermCmd::Say(state))
}

pub fn cmd_exit(_args: &ArgsMap) -> Result<TermCmd, ParseError> {
    Ok(TermCmd::Exit)
}

pub fn cmd_clear(_args: &ArgsMap) -> Result<TermCmd, ParseError> {
    Ok(TermCmd::Clear)
}

pub fn client_request_blob_commands() -> Vec<Command<TermCmd>> {
    TableIdent::all()
        .map(|t| {
            let name = format!("client.req.blob.{t}").to_lowercase();
            Command::new(&name, vec![], move |_| Ok(TermCmd::ClientReqBlob(t)))
        })
        .chain([Command::new("client.req.blob.all", vec![], |_| {
            Ok(TermCmd::ClientReqAllBlobs)
        })])
        .collect()
}

pub fn blob_info_commands() -> Vec<Command<TermCmd>> {
    TableIdent::all()
        .map(|t| {
            let name = format!("print.blob.{t}").to_lowercase();
            Command::new(&name, vec![], move |_| Ok(TermCmd::PrintBlobInfo(t)))
        })
        .collect()
}

pub fn load_command() -> Command<TermCmd> {
    Command::new("load", vec!["path"], |args| {
        let save = parse_arg(args, "path")?;
        Ok(TermCmd::LoadSave(save))
    })
}

pub fn save_command() -> Command<TermCmd> {
    Command::new("save", vec!["savename", "overwrite"], |args| {
        let savename = parse_arg(args, "savename")?;
        let overwrite = parse_arg(args, "overwrite")?;
        Ok(TermCmd::SaveWorldToDisk(savename, overwrite))
    })
}

pub fn help_command() -> Command<TermCmd> {
    Command::new("help", vec![], |_| Ok(TermCmd::Help))
}

pub fn pwd_command() -> Command<TermCmd> {
    Command::new("pwd", vec![], |_| Ok(TermCmd::Pwd))
}

pub fn cd_command() -> Command<TermCmd> {
    Command::new("cd", vec!["dirname"], |args| {
        let dirname = parse_arg(args, "dirname")?;
        Ok(TermCmd::Cd(dirname))
    })
}

pub fn loglevel_command() -> Command<TermCmd> {
    Command::new("term.debug", vec![], |_| Ok(TermCmd::ToggleDebugMode))
}

pub fn terminal_commands() -> Vec<Command<TermCmd>> {
    vec![help_command(), loglevel_command(), cd_command()]
}

pub fn server_console_commands() -> Vec<Command<TermCmd>> {
    vec![
        Command::new("echo", vec![], |_| Ok(TermCmd::PrintSaveInfo)),
        load_command(),
        save_command(),
        Command::new("clear", vec![], cmd_clear),
        Command::new("speed", vec!["speed"], cmd_set_speed),
        Command::new("ls.saves", vec![], |_| Ok(TermCmd::ListSaves)),
        Command::new("ls.blueprints", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Blueprints))
        }),
        Command::new("ls.grids", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Grids))
        }),
        Command::new("ls.protos", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Protos))
        }),
        Command::new("ls.parts", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Parts))
        }),
        Command::new("ls.thrusters", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Thrusters))
        }),
        Command::new("ls.cpus", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Computers))
        }),
    ]
}

pub fn world_delta_commands() -> Vec<Command<TermCmd>> {
    vec![
        Command::new(
            "sim.spawn",
            vec!["ship_name", "bp_name", "bp_version", "x", "y"],
            |args| {
                let name = parse_arg(args, "ship_name")?;
                let bpname = parse_arg(args, "bp_name")?;
                let version = parse_arg(args, "bp_version")?;
                let x = parse_arg(args, "x")?;
                let y = parse_arg(args, "y")?;
                Ok(TermCmd::World(WorldDelta::SpawnShipAt(
                    name,
                    BlueprintId(bpname, version),
                    Isometry2d::from_pos(Vec2::new(x, y)),
                )))
            },
        ),
        Command::new("sim.despawn", vec!["grid_id"], |args| {
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            Ok(TermCmd::World(WorldDelta::DespawnGrid(grid_id)))
        }),
        Command::new("sim.ping", vec!["x", "y"], |args| {
            let x = parse_arg(args, "x")?;
            let y = parse_arg(args, "y")?;
            Ok(TermCmd::World(WorldDelta::Ping((x, y).into())))
        }),
        Command::new("sim.clear", vec![], |_| {
            Ok(TermCmd::World(WorldDelta::ClearAll))
        }),
        Command::new("sim.waypoint", vec!["grid_id", "x", "y"], |args| {
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            let x = parse_arg(args, "x")?;
            let y = parse_arg(args, "y")?;
            Ok(TermCmd::World(WorldDelta::SetWaypoint {
                grid_id,
                waypoint: Isometry2d::from_xya(x, y, 0.0),
            }))
        }),
        Command::new("sim.explode", vec!["grid_id", "x", "y"], |args| {
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            let x = parse_arg(args, "x")?;
            let y = parse_arg(args, "y")?;
            let loc = GridLocation::new(grid_id, (x, y).into());
            Ok(TermCmd::World(WorldDelta::Explode(loc)))
        }),
        Command::new("sim.toggle_tracking", vec!["grid_id"], |args| {
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            Ok(TermCmd::World(WorldDelta::ToggleTracking(grid_id)))
        }),
        Command::new("sim.ast.remove", vec!["ast_id", "x", "y"], |args| {
            let ast_id = Ent(parse_arg(args, "ast_id")?);
            let x = parse_arg(args, "x")?;
            let y = parse_arg(args, "y")?;
            Ok(TermCmd::World(WorldDelta::RemoveTerrainTile {
                asteroid: ast_id,
                tile: GlobalTileIndex((x, y).into()),
            }))
        }),
        Command::new("sim.ast.add", vec!["ast_id", "x", "y"], |args| {
            let ast_id = Ent(parse_arg(args, "ast_id")?);
            let x = parse_arg(args, "x")?;
            let y = parse_arg(args, "y")?;
            Ok(TermCmd::World(WorldDelta::AddTerrainTile {
                asteroid: ast_id,
                tile: GlobalTileIndex((x, y).into()),
                material: TerrainMaterial::Rock,
            }))
        }),
        Command::new("sim.ast.reveal", vec!["ast_id", "x", "y"], |args| {
            let ast_id = Ent(parse_arg(args, "ast_id")?);
            let x = parse_arg(args, "x")?;
            let y = parse_arg(args, "y")?;
            Ok(TermCmd::World(WorldDelta::FullyRevealTerrainTile {
                asteroid: ast_id,
                tile: GlobalTileIndex((x, y).into()),
            }))
        }),
        Command::new("sim.ast.spawn", vec!["x", "y", "r", "seed"], |args| {
            let x: f32 = parse_arg(args, "x")?;
            let y: f32 = parse_arg(args, "y")?;
            let r = parse_arg(args, "r")?;
            let seed = parse_arg(args, "seed")?;
            Ok(TermCmd::World(WorldDelta::SpawnAsteroid {
                iso: (x, y, 0.0).into(),
                radius: r,
                seed,
            }))
        }),
        Command::new("sim.goto.ast", vec!["grid_id", "ast_id"], |args| {
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            let ast_id = Ent(parse_arg(args, "ast_id")?);
            Ok(TermCmd::World(WorldDelta::GoToAsteroid { grid_id, ast_id }))
        }),
        Command::new("sim.anchor", vec!["grid_id", "is_anchored"], |args| {
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            let is_anchored = parse_arg(args, "is_anchored")?;
            Ok(TermCmd::World(WorldDelta::SetAnchored(
                grid_id,
                is_anchored,
            )))
        }),
        Command::new("sim.pilot.enter", vec!["grid_id"], |args| {
            let player_id = Ent(parse_arg(args, "player_id")?);
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            Ok(TermCmd::World(WorldDelta::PlayerPilotingEnterGrid {
                player_id,
                grid_id,
            }))
        }),
    ]
}

pub fn all_commands() -> Vec<Command<TermCmd>> {
    vec![
        Command::new("find", vec!["grid_name"], cmd_find),
        Command::new("speed", vec!["speed"], cmd_set_speed),
        Command::new("save", vec![], cmd_placeholder),
        Command::new("exit", vec![], cmd_exit),
        Command::new("say", vec!["msg"], cmd_say),
        Command::new("clear", vec![], cmd_clear),
        Command::new("ls.grids", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Grids))
        }),
        Command::new("ls.blueprints", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Blueprints))
        }),
        Command::new("ls.protos", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Protos))
        }),
        Command::new("ls.parts", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Parts))
        }),
        Command::new("ls.thrusters", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Thrusters))
        }),
        Command::new("ls.cpus", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Computers))
        }),
        Command::new("server.disconnect", vec![], |_| {
            Ok(TermCmd::ServerDisconnect)
        }),
        Command::new("server.connect", vec![], |_| Ok(TermCmd::ServerConnect)),
        Command::new("server.info", vec![], |_| {
            Ok(TermCmd::RequestServerStatistics)
        }),
    ]
}
