use bevy::prelude::*;
use bevy::render::mesh::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::{na::Point2, prelude::*};
use log::{debug, error, info, trace, warn};
pub struct TerrainMeshPlugin;

impl Plugin for TerrainMeshPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(spawn_meshes.system())
            .add_startup_system(spawn_finish_flag.system());
    }
}

fn spawn_finish_flag(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cmd: Commands,
) {
    let finish_tex = asset_server.load("textures/finish.png");
    let scale = 1.5;

    let transform = Transform {
        scale: Vec3::ONE * scale,
        translation: Vec3::new(14600., -500., 60.),
        ..Default::default()
    };
    let sprite = SpriteBundle {
        material: materials.add(ColorMaterial {
            texture: finish_tex.into(),
            ..Default::default()
        }),
        transform,
        ..Default::default()
    };

    cmd.spawn_bundle(sprite);
}

fn spawn_meshes(
    mut cmd: Commands,
    mut ev_asset: EventReader<AssetEvent<Mesh>>,
    assets: ResMut<Assets<Mesh>>,
    config: Res<RapierConfiguration>,
) {
    for ev in ev_asset.iter() {
        if let AssetEvent::Created { handle } = ev {
            let mesh = assets.get(handle).unwrap();

            let attr_pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION);

            let terrain_handle = assets
                .get_handle("models/TerrainRoad.glb#Mesh0/Primitive0")
                .clone();

            if *handle == terrain_handle {
                let attr_pos = attr_pos.unwrap();

                let indices = mesh.indices().expect("mesh has no index buffer");
                let indices = match indices {
                    Indices::U32(indices) => indices,
                    _ => {
                        panic!("mesh indices aren't u32");
                    }
                };

                let pos_to_2d = |[x, _y, z]: [f32; 3]| [x, -z];

                match attr_pos {
                    VertexAttributeValues::Float3(positions) => {
                        for [x, y, z] in positions {
                            trace!("{}, {}, {}", x, y, z);
                        }

                        let mut triangles_2d = vec![];

                        for i in indices.chunks_exact(3) {
                            let i1 = i[0] as usize;
                            let i2 = i[1] as usize;
                            let i3 = i[2] as usize;

                            let v1 = pos_to_2d(positions[i1]);
                            let v2 = pos_to_2d(positions[i2]);
                            let v3 = pos_to_2d(positions[i3]);

                            triangles_2d.push((v1, v2, v3));
                        }

                        build_terrain(&mut cmd, &triangles_2d, &config);
                    }
                    _ => {
                        warn!("position wasn't a Float3");
                    }
                }
            }
        }
    }
}

fn build_terrain(
    cmd: &mut Commands,
    triangles: &[([f32; 2], [f32; 2], [f32; 2])],
    config: &RapierConfiguration,
) {
    let mut builder1 = GeometryBuilder::new();
    let mut builder2 = GeometryBuilder::new(); //can't clone a Builder :(

    for (v1, v2, v3) in triangles.iter() {
        let a = Vec2::new(v1[0], v1[1]);
        let b = Vec2::new(v2[0], v2[1]);
        let c = Vec2::new(v3[0], v3[1]);

        let points = vec![a, b, c];
        let tri = shapes::Polygon {
            points,
            closed: true,
        };
        builder1.add(&tri);
        builder2.add(&tri);
    }

    let fill_options = FillOptions::DEFAULT;
    let outline_options = StrokeOptions::default().with_line_width(12.);
    let shape_bundle_fill = builder1.build(
        ShapeColors::new(Color::rgb(0.258, 0.525, 0.67)),
        DrawMode::Fill(fill_options),
        Transform::from_translation(Vec3::new(0., 0., 50.)),
    );

    let shape_bundle_outline = builder2.build(
        ShapeColors::new(Color::rgba(0., 0., 0., 0.3)),
        DrawMode::Stroke(outline_options),
        Transform::from_translation(Vec3::new(0., 0., 40.)),
    );

    let rigid_body = RigidBodyBundle {
        body_type: RigidBodyType::Static,
        ..Default::default()
    };

    let triangle_shapes = triangles
        .iter()
        .map(|(v1, v2, v3)| {
            let a = Point2::new(v1[0], v1[1]) / config.scale;
            let b = Point2::new(v2[0], v2[1]) / config.scale;
            let c = Point2::new(v3[0], v3[1]) / config.scale;

            (Isometry::identity(), ColliderShape::triangle(a, b, c))
        })
        .collect::<Vec<_>>();
    let collider_shape = ColliderShape::compound(triangle_shapes);

    let collider = ColliderBundle {
        shape: collider_shape,
        material: ColliderMaterial::new(0.3, 0.1),

        ..Default::default()
    };

    let debug_draw = false; //TODO debug draw seems broken?
    let mut ent = cmd.spawn_bundle(shape_bundle_fill);
    ent.insert_bundle(collider)
        .insert_bundle(rigid_body)
        .insert(RigidBodyPositionSync::Discrete)
        .insert(Name::new("Terrain".to_owned()));

    if debug_draw {
        ent.insert(ColliderDebugRender::with_id(1))
            .insert(ColliderPositionSync::Discrete);
    }

    //Seperate entity for the outline
    cmd.spawn_bundle(shape_bundle_outline)
        .insert(Name::new("TerrainOutlines".to_owned()));
}
