use log::warn;
use serde::{Deserialize, Serialize};

use amethyst_assets::{AssetStorage, PrefabData, ProgressCounter};
use amethyst_core::{
    ecs::{Entity, Read, Write, WriteStorage},
    Transform,
};
use amethyst_error::Error;

use crate::{Sprite, SpriteRender, SpriteSheet, SpriteSheetHandle, TextureFormat, TexturePrefab};

/// Represents one sprite in `SpriteList`.
/// Positions originate in the top-left corner (bitmap image convention).
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SpritePosition {
    /// Horizontal position of the sprite in the sprite sheet
    pub x: u32,
    /// Vertical position of the sprite in the sprite sheet
    pub y: u32,
    /// Width of the sprite
    pub width: u32,
    /// Height of the sprite
    pub height: u32,
    /// Number of pixels to shift the sprite to the left and down relative to the entity holding it
    pub offsets: Option<[f32; 2]>,
}

/// `SpriteList` controls how a sprite list is generated when using `Sprites::List` in a
/// `SpriteSheetPrefab`.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SpriteList {
    /// Width of the sprite sheet
    #[serde(rename = "spritesheet_width")]
    pub width: u32,
    /// Height of the sprite sheet
    #[serde(rename = "spritesheet_height")]
    pub height: u32,
    /// Description of the sprites
    pub sprites: Vec<SpritePosition>,
}

/// `SpriteGrid` controls how a sprite grid is generated when using `Sprites::Grid` in a
/// `SpriteSheetPrefab`.
///
/// The number of columns in the grid must always be provided, and one of the other fields must also
/// be provided. The grid will be layout row major, starting with the sprite in the upper left corner,
/// and ending with the sprite in the lower right corner. For example a grid with 2 rows and 4 columns
/// will have the order below for the sprites.
///
/// ```text
/// |---|---|---|---|
/// | 0 | 1 | 2 | 3 |
/// |---|---|---|---|
/// | 4 | 5 | 6 | 7 |
/// |---|---|---|---|
/// ```
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct SpriteGrid {
    /// Height of the spritesheet in pixels.
    #[serde(rename = "spritesheet_width")]
    pub width: u32,
    /// Width of the spritesheet in pixels.
    #[serde(rename = "spritesheet_height")]
    pub height: u32,
    /// Specifies the number of columns in the spritesheet, this value must always be given.
    pub columns: u32,
    /// Specifies the number of rows in the spritesheet. If this is not given it will be calculated
    /// using either `count` (`count / columns`), or `cell_size` (`sheet_size / cell_size`).
    pub rows: Option<u32>,
    /// Specifies the number of sprites in the spritesheet. If this is not given it will be
    /// calculated using `rows` (`columns * rows`).
    pub count: Option<u32>,
    /// Specifies the size of the individual sprites in the spritesheet in pixels. If this is not
    /// given it will be calculated using the spritesheet size, `columns` and `rows`.
    /// Tuple order is `(width, height)`.
    pub cell_size: Option<(u32, u32)>,
    /// Specifies the position of the grid on a texture. If this is not given it will be set to (0, 0).
    /// Positions originate in the top-left corner (bitmap image convention).
    pub position: Option<(u32, u32)>,
}

/// Defined the sprites that are part of a `SpriteSheetPrefab`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Sprites {
    /// A list of sprites
    List(SpriteList),
    /// Generate a grid sprite list, see `SpriteGrid` for more information.
    Grid(SpriteGrid),
}

/// Defines a spritesheet prefab. Note that this prefab will only load the spritesheet in storage,
/// no components will be added to entities. The `add_to_entity` will return the
/// `Handle<SpriteSheet>`. For this reason it is recommended that this prefab is only used as part
/// of other `PrefabData` or in specialised formats. See `SpriteScenePrefab` for an example of this.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SpriteSheetPrefab {
    /// Spritesheet handle, normally not used outside the prefab system.
    #[serde(skip)]
    Handle(SpriteSheetHandle),
    /// Definition of a spritesheet
    Sheet {
        /// This texture contains the images for the spritesheet
        texture: TexturePrefab<TextureFormat>,
        /// The sprites in the spritesheet
        sprites: Vec<Sprites>,
    },
}

impl<'a> PrefabData<'a> for SpriteSheetPrefab {
    type SystemData = (
        <TexturePrefab<TextureFormat> as PrefabData<'a>>::SystemData,
        Read<'a, AssetStorage<SpriteSheet>>,
    );
    type Result = SpriteSheetHandle;

    fn add_to_entity(
        &self,
        _entity: Entity,
        _system_data: &mut Self::SystemData,
        _entities: &[Entity],
    ) -> Result<Self::Result, Error> {
        match self {
            SpriteSheetPrefab::Handle(handle) => Ok(handle.clone()),
            _ => unreachable!(),
        }
    }

    fn load_sub_assets(
        &mut self,
        progress: &mut ProgressCounter,
        system_data: &mut Self::SystemData,
    ) -> Result<bool, Error> {
        let handle = match self {
            SpriteSheetPrefab::Sheet { texture, sprites } => {
                texture.load_sub_assets(progress, &mut system_data.0)?;
                let texture_handle = match texture {
                    TexturePrefab::Handle(handle) => handle.clone(),
                    _ => unreachable!(),
                };
                let sprites = sprites.iter().flat_map(|s| s.build_sprites()).collect();
                let spritesheet = SpriteSheet {
                    texture: texture_handle,
                    sprites,
                };
                Some(
                    (system_data.0)
                        .0
                        .load_from_data(spritesheet, progress, &system_data.1),
                )
            }
            _ => None,
        };
        match handle {
            Some(handle) => {
                *self = SpriteSheetPrefab::Handle(handle);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct SpriteSheetLoadedSet(pub Vec<SpriteSheetHandle>);

/// Prefab used to add a sprite to an `Entity`.
///
/// This prefab is special in that it will lookup the spritesheet in the resource
/// `SpriteSheetLoadedSet` by index during loading. Just like with `SpriteSheetPrefab` this means
/// that this prefab should only be used as part of other prefabs or in specialised formats. Look at
/// `SpriteScenePrefab` for an example.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct SpriteRenderPrefab {
    /// Index of the sprite sheet in the prefab
    pub sheet: usize,
    /// Index of the sprite on the sprite sheet
    pub sprite_number: usize,

    #[serde(skip)]
    handle: Option<SpriteSheetHandle>,
}

impl<'a> PrefabData<'a> for SpriteRenderPrefab {
    type SystemData = (
        WriteStorage<'a, SpriteRender>,
        Write<'a, SpriteSheetLoadedSet>,
    );
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        system_data: &mut <Self as PrefabData<'a>>::SystemData,
        _: &[Entity],
    ) -> Result<(), Error> {
        system_data.0.insert(
            entity,
            SpriteRender {
                sprite_sheet: self.handle.as_ref().unwrap().clone(),
                sprite_number: self.sprite_number,
            },
        )?;
        Ok(())
    }

    fn load_sub_assets(
        &mut self,
        _: &mut ProgressCounter,
        system_data: &mut Self::SystemData,
    ) -> Result<bool, Error> {
        self.handle = Some((system_data.1).0.get(self.sheet).cloned().unwrap());
        Ok(false)
    }
}

/// Prefab for loading a full scene with sprites.
///
/// When a `render` is encountered during the processing of this prefab, the sheet index
/// in that will be loaded from the last encountered `sheets`. It is therefore recommended that
/// all sheets used in the prefab be loaded on the first entity only.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpriteScenePrefab {
    /// Sprite sheets
    pub sheet: Option<SpriteSheetPrefab>,
    /// Add `SpriteRender` to the `Entity`
    pub render: Option<SpriteRenderPrefab>,
    /// Add `Transform` to the `Entity`
    pub transform: Option<Transform>,
}

impl<'a> PrefabData<'a> for SpriteScenePrefab {
    type SystemData = (
        <SpriteSheetPrefab as PrefabData<'a>>::SystemData,
        <SpriteRenderPrefab as PrefabData<'a>>::SystemData,
        <Transform as PrefabData<'a>>::SystemData,
    );
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        system_data: &mut Self::SystemData,
        entities: &[Entity],
    ) -> Result<(), Error> {
        if let Some(render) = &self.render {
            render.add_to_entity(entity, &mut system_data.1, entities)?;
        }
        if let Some(transform) = &self.transform {
            transform.add_to_entity(entity, &mut system_data.2, entities)?;
        }
        Ok(())
    }

    fn load_sub_assets(
        &mut self,
        progress: &mut ProgressCounter,
        system_data: &mut Self::SystemData,
    ) -> Result<bool, Error> {
        let mut ret = false;
        if let Some(ref mut sheet) = &mut self.sheet {
            if sheet.load_sub_assets(progress, &mut system_data.0)? {
                ret = true;
            }
            let sheet = match sheet {
                SpriteSheetPrefab::Handle(handle) => handle.clone(),
                _ => unreachable!(),
            };
            ((system_data.1).1).0.push(sheet);
        }
        if let Some(ref mut render) = &mut self.render {
            render.load_sub_assets(progress, &mut system_data.1)?;
        }
        Ok(ret)
    }
}

impl Sprites {
    fn build_sprites(&self) -> Vec<Sprite> {
        match self {
            Sprites::List(list) => list.build_sprites(),
            Sprites::Grid(grid) => grid.build_sprites(),
        }
    }
}

impl SpriteList {
    pub(crate) fn build_sprites(&self) -> Vec<Sprite> {
        self.sprites
            .iter()
            .map(|pos| {
                Sprite::from_pixel_values(
                    self.width,
                    self.height,
                    pos.width,
                    pos.height,
                    pos.x,
                    pos.y,
                    pos.offsets.unwrap_or([0.0; 2]),
                )
            })
            .collect()
    }
}

impl SpriteGrid {
    fn width(&self) -> u32 {
        self.width - self.position().0
    }

    fn height(&self) -> u32 {
        self.height - self.position().1
    }

    fn rows(&self) -> u32 {
        self.rows.unwrap_or_else(|| {
            self.count
                .map(|c| {
                    if (c % self.columns) == 0 {
                        (c / self.columns)
                    } else {
                        (c / self.columns) + 1
                    }
                })
                .or_else(|| self.cell_size.map(|(_, y)| (self.height() / y)))
                .unwrap_or(1)
        })
    }

    fn count(&self) -> u32 {
        self.count.unwrap_or_else(|| self.columns * self.rows())
    }

    fn cell_size(&self) -> (u32, u32) {
        self.cell_size
            .unwrap_or_else(|| ((self.width() / self.columns), (self.height() / self.rows())))
    }

    fn position(&self) -> (u32, u32) {
        self.position.unwrap_or((0, 0))
    }

    fn build_sprites(&self) -> Vec<Sprite> {
        let rows = self.rows();
        let count = self.count();
        let cell_size = self.cell_size();
        let position = self.position();
        if (self.columns * cell_size.0) > self.width() {
            warn!(
                "Grid spritesheet contain more columns than can fit in the given width: {} * {} > {} - {}",
                self.columns,
                cell_size.0,
                self.width,
                position.0
            );
        }
        if (rows * cell_size.1) > self.height() {
            warn!(
                "Grid spritesheet contain more rows than can fit in the given height: {} * {} > {} - {}",
                rows,
                cell_size.1,
                self.height,
                position.1
            );
        }
        (0..count)
            .map(|cell| {
                let row = cell / self.columns;
                let column = cell - (row * self.columns);
                let x = column * cell_size.0 + position.0;
                let y = row * cell_size.1 + position.1;
                Sprite::from_pixel_values(
                    self.width,
                    self.height,
                    cell_size.0,
                    cell_size.1,
                    x,
                    y,
                    [0.0; 2],
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Texture;
    use amethyst_assets::{Handle, Loader};
    use amethyst_core::ecs::{Builder, Read, ReadExpect, World};
    use rayon::ThreadPoolBuilder;
    use std::sync::Arc;

    fn setup_sprite_world() -> World {
        let mut world = World::new();
        world.register::<SpriteRender>();
        let loader = Loader::new(".", Arc::new(ThreadPoolBuilder::new().build().unwrap()));
        let tex_storage = AssetStorage::<Texture>::default();
        let ss_storage = AssetStorage::<SpriteSheet>::default();
        world.add_resource(tex_storage);
        world.add_resource(ss_storage);
        world.add_resource(SpriteSheetLoadedSet::default());
        world.add_resource(loader);
        world
    }

    fn add_sheet(world: &mut World) -> (usize, Handle<SpriteSheet>) {
        type Data<'a> = (
            ReadExpect<'a, Loader>,
            Read<'a, AssetStorage<SpriteSheet>>,
            Write<'a, SpriteSheetLoadedSet>,
        );
        let texture = add_texture(world);
        world.exec(|mut data: Data<'_>| {
            let spritesheet = data.0.load_from_data(
                SpriteSheet {
                    texture,
                    sprites: vec![],
                },
                (),
                &data.1,
            );
            let index = (data.2).0.len();
            (data.2).0.push(spritesheet.clone());
            (index, spritesheet)
        })
    }

    fn add_texture(world: &mut World) -> Handle<Texture> {
        type Data<'a> = (ReadExpect<'a, Loader>, Read<'a, AssetStorage<Texture>>);
        world.exec(|data: Data<'_>| data.0.load_from_data([1., 1., 1., 1.].into(), (), &data.1))
    }

    #[test]
    fn sprite_sheet_prefab() {
        let mut world = setup_sprite_world();
        let texture = add_texture(&mut world);
        let mut prefab = SpriteSheetPrefab::Sheet {
            sprites: vec![Sprites::List(SpriteList {
                width: 0,
                height: 0,
                sprites: vec![],
            })],
            texture: TexturePrefab::Handle(texture.clone()),
        };
        prefab
            .load_sub_assets(&mut ProgressCounter::default(), &mut world.system_data())
            .unwrap();
        let h = match &prefab {
            SpriteSheetPrefab::Handle(h) => h.clone(),
            _ => {
                assert!(false);
                return;
            }
        };
        let entity = world.create_entity().build();
        let handle = prefab
            .add_to_entity(entity, &mut world.system_data(), &[entity])
            .unwrap();
        assert_eq!(h, handle);
    }

    #[test]
    fn sprite_render_prefab() {
        let mut world = setup_sprite_world();
        let (sheet, handle) = add_sheet(&mut world);
        let entity = world.create_entity().build();
        let mut prefab = SpriteRenderPrefab {
            sheet,
            sprite_number: 0,
            handle: None,
        };
        prefab
            .load_sub_assets(&mut ProgressCounter::default(), &mut world.system_data())
            .unwrap();
        prefab
            .add_to_entity(entity, &mut world.system_data(), &[entity])
            .unwrap();
        let storage = world.read_storage::<SpriteRender>();
        let render = storage.get(entity);
        assert!(render.is_some());
        let render = render.unwrap();
        assert_eq!(0, render.sprite_number);
        assert_eq!(handle, render.sprite_sheet);
    }

    #[test]
    fn grid_col_row() {
        let sprites = SpriteGrid {
            width: 400,
            height: 200,
            columns: 4,
            rows: Some(4),
            ..Default::default()
        }
        .build_sprites();

        assert_eq!(16, sprites.len());
        for sprite in &sprites {
            assert_eq!(50., sprite.height);
            assert_eq!(100., sprite.width);
            assert_eq!([0., 0.], sprite.offsets);
        }
        assert_eq!(0., sprites[0].tex_coords.left);
        assert_eq!(0.25, sprites[0].tex_coords.right);
        assert_eq!(1.0, sprites[0].tex_coords.top);
        assert_eq!(0.75, sprites[0].tex_coords.bottom);

        assert_eq!(0.75, sprites[7].tex_coords.left);
        assert_eq!(1.0, sprites[7].tex_coords.right);
        assert_eq!(0.75, sprites[7].tex_coords.top);
        assert_eq!(0.5, sprites[7].tex_coords.bottom);

        assert_eq!(0.25, sprites[9].tex_coords.left);
        assert_eq!(0.5, sprites[9].tex_coords.right);
        assert_eq!(0.5, sprites[9].tex_coords.top);
        assert_eq!(0.25, sprites[9].tex_coords.bottom);

        let sprites = SpriteGrid {
            width: 192,
            height: 64,
            columns: 6,
            rows: Some(2),
            ..Default::default()
        }
        .build_sprites();

        assert_eq!(12, sprites.len());
        for sprite in &sprites {
            assert_eq!(32.0, sprite.height);
            assert_eq!(32.0, sprite.width);
            assert_eq!([0.0, 0.0], sprite.offsets);
        }
        assert_eq!(0.0, sprites[0].tex_coords.left);
        assert_eq!(0.16666667, sprites[0].tex_coords.right);
        assert_eq!(1.0, sprites[0].tex_coords.top);
        assert_eq!(0.5, sprites[0].tex_coords.bottom);

        assert_eq!(0.16666667, sprites[7].tex_coords.left);
        assert_eq!(0.33333334, sprites[7].tex_coords.right);
        assert_eq!(0.5, sprites[7].tex_coords.top);
        assert_eq!(0.0, sprites[7].tex_coords.bottom);

        assert_eq!(0.5, sprites[9].tex_coords.left);
        assert_eq!(0.6666667, sprites[9].tex_coords.right);
        assert_eq!(0.5, sprites[9].tex_coords.top);
        assert_eq!(0.0, sprites[9].tex_coords.bottom);
    }

    #[test]
    fn grid_position() {
        let sprites = SpriteGrid {
            width: 192,
            height: 64,
            columns: 5,
            rows: Some(1),
            position: Some((32, 32)),
            ..Default::default()
        }
        .build_sprites();

        assert_eq!(5, sprites.len());
        for sprite in &sprites {
            assert_eq!(32.0, sprite.height);
            assert_eq!(32.0, sprite.width);
            assert_eq!([0.0, 0.0], sprite.offsets);
        }

        assert_eq!(0.16666667, sprites[0].tex_coords.left);
        assert_eq!(0.33333334, sprites[0].tex_coords.right);
        assert_eq!(0.5, sprites[0].tex_coords.top);
        assert_eq!(0.0, sprites[0].tex_coords.bottom);

        assert_eq!(0.8333333, sprites[4].tex_coords.left);
        assert_eq!(1.0, sprites[4].tex_coords.right);
        assert_eq!(0.5, sprites[4].tex_coords.top);
        assert_eq!(0.0, sprites[4].tex_coords.bottom);
    }

    #[test]
    fn repeat_cell_size_set() {
        assert_eq!(
            (100, 100),
            SpriteGrid {
                width: 200,
                height: 200,
                columns: 4,
                cell_size: Some((100, 100)),
                ..Default::default()
            }
            .cell_size()
        );
    }

    #[test]
    fn repeat_cell_size_no_set() {
        assert_eq!(
            (50, 100),
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 4,
                rows: Some(4),
                ..Default::default()
            }
            .cell_size()
        );
    }

    #[test]
    fn repeat_count_count_set() {
        assert_eq!(
            12,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                count: Some(12),
                ..Default::default()
            }
            .count()
        );
    }

    #[test]
    fn repeat_count_no_set() {
        assert_eq!(
            10,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                rows: Some(2),
                ..Default::default()
            }
            .count()
        );
    }

    #[test]
    fn repeat_rows_rows_set() {
        assert_eq!(
            5,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                rows: Some(5),
                ..Default::default()
            }
            .rows()
        );
    }

    #[test]
    fn repeat_rows_count_set() {
        assert_eq!(
            3,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                count: Some(12),
                ..Default::default()
            }
            .rows()
        );
        assert_eq!(
            3,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                count: Some(15),
                ..Default::default()
            }
            .rows()
        );
    }

    #[test]
    fn repeat_rows_cell_size_set() {
        assert_eq!(
            2,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                cell_size: Some((200, 200)),
                ..Default::default()
            }
            .rows()
        );
        assert_eq!(
            2,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                cell_size: Some((150, 150)),
                ..Default::default()
            }
            .rows()
        );
        assert_eq!(
            2,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                cell_size: Some((199, 199)),
                ..Default::default()
            }
            .rows()
        );
    }

    #[test]
    fn repeat_rows_cell_no_set() {
        assert_eq!(
            1,
            SpriteGrid {
                width: 200,
                height: 400,
                columns: 5,
                ..Default::default()
            }
            .rows()
        );
    }
}
