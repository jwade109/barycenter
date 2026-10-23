use bary_core::prelude::*;
use bary_factory::*;
use bary_sim::*;

fn scenario_a() -> BaryResult<()> {
    let world = WorldBuilder::new()
        .assets()
        .planet("Lado", Isometry2d::ZERO, 3E15, 100.0)
        .blueprint(("pollux", 0))
        .blueprint(("pollux", 2))
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .blueprint("foundation")
        .blueprint("miner")
        .blueprint("icecream")
        .spawn(("pollux", 0), "", (1030.0, 0.0, 0.0))
        .set_velocity((0.0, 13.0))
        .spawn(("pollux", 2), "", (1100.0, 0.0, 0.0))
        .set_velocity((0.0, 13.0))
        .insert_source((19, 7), Item::Magnesium)
        .insert_source((20, 7), Item::Iron)
        .insert_source((21, 7), Item::Titanium)
        .set_recipe((21, 7), RecipeListing::TitaniumLattice)
        .insert_source((22, 11), Item::Water)
        .insert_pipe((22, 11), (22, 10))
        .set_recipe((27, 7), RecipeListing::WaterElectrolysis)
        .insert_pipe((17, 10), (15, 10))
        .insert_pipe((27, 7), (34, 7))
        .insert_pipe((26, 7), (25, 6))
        .spawn("remora", "", (1010.0, 30.0, 0.1))
        .set_velocity((0.0, 12.0))
        .spawn("miner", "", (1109.0, 12.0, -0.3))
        .set_velocity((0.0, 12.0))
        .spawn("remora", "", (1300.0, 23.0, 0.7))
        .set_velocity((0.0, 12.0))
        .spawn("bellerophon", "", (1309.0, 50.0, 0.1))
        .set_velocity((0.0, 12.0))
        .command(WorldDelta::Ping(DVec2::ZERO))
        .command(WorldDelta::Ping(DVec2::splat(10.0)))
        .asteroid((-80.0, 30.0, 0.1), 20.0, 632)
        .asteroid((60.0, 300.0, 0.7), 50.0, 2384)
        // .asteroid((400.0, -2000.0, 0.7), 500.0, 9312)
        .build();

    save_world("saves/scenario_a", &world, true)
}

fn scenario_b() -> BaryResult<()> {
    let world = WorldBuilder::new()
        .assets()
        .blueprint(("pollux", 0))
        .blueprint(("pollux", 2))
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .blueprint("foundation")
        .blueprint("miner")
        .blueprint("icecream")
        .blueprint("miner")
        .spawn(("pollux", 2), "", (0.0, 0.0, 0.0))
        .spawn("remora", "", (50.0, 50.0, 0.0))
        .spawn("miner", "", (-20.0, 20.0, 4.0))
        .asteroid((-80.0, 30.0, 0.1), 50.0, 391)
        .build();

    save_world("saves/scenario_b", &world, true)
}

fn scenario_c() -> BaryResult<()> {
    let world = WorldBuilder::new()
        .assets()
        .parse_file("scenario_c.yaml")
        .build();

    save_world("saves/scenario_c", &world, true)
}

fn main() -> BaryResult<()> {
    simple_logger::init_with_level(log::Level::Trace).unwrap();

    scenario_a()?;
    scenario_b()?;
    scenario_c()?;
    Ok(())
}
