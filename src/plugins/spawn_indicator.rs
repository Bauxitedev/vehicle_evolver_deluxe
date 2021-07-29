use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use log::{debug, error, info, trace, warn};

//NOTE: I dunno why but this plugin is vital for WASM.
//If you disable it, the terrain mesh becomes broken!

pub struct SpawnIndicatorPlugin;
impl Plugin for SpawnIndicatorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_shape.system());
    }
}

fn setup_shape(mut commands: Commands) {
    let mut points = vec![];

    let x = 300.;
    let y = 300.;

    points.push(Vec2::new(x, y));
    points.push(Vec2::new(x, -y));
    points.push(Vec2::new(-x, -y));
    points.push(Vec2::new(-x, y));
    points.push(Vec2::new(x, y));

    let lines = points
        .windows(2)
        .map(|pair| {
            let a = pair[0];
            let b = pair[1];

            shapes::Line(a, b)
        })
        .collect::<Vec<_>>();

    let mut builder = GeometryBuilder::new();
    for line in &lines {
        builder.add(line);
    }

    let stroke = StrokeOptions::default().with_line_width(1.);
    let shape_bundle = builder.build(
        ShapeColors::outlined(Color::WHITE, Color::WHITE),
        DrawMode::Stroke(stroke),
        Transform::from_translation(Vec3::new(0., 200., 50.)),
    );

    commands
        .spawn_bundle(shape_bundle)
        .insert(Name::new("Spawn".to_owned()));
}
