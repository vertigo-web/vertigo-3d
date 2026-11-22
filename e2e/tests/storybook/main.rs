//! The vertigo-3d storybook, story by story. Every test gets a fresh page, and the stories keep
//! their state in the page only, so the tests can run in parallel.

mod background;
mod cubes;
mod dragging;
mod glass;
mod layers;
mod light;
mod picking;
mod relief;
mod shaders;
mod spinning;
mod textures;
mod thousands;

use vertigo_3d_e2e::{browser_tests, env::suite, prelude::*};

fn main() {
    let tests = browser_tests![
        background::scene_shows_its_background,
        background::the_scenes_are_drawn_as_the_run_draws,
        background::scene_draws_again_when_the_background_changes,
        cubes::the_ball_hides_and_shows_again,
        cubes::the_camera_goes_round,
        dragging::a_box_goes_with_the_pointer_along_the_floor,
        dragging::a_box_stops_at_the_wall,
        dragging::a_drag_goes_on_outside_the_canvas,
        dragging::the_pointer_shows_a_box_can_be_grabbed,
        glass::the_nearer_pane_shows_more_of_its_color,
        glass::a_pane_tints_what_is_behind_it,
        glass::a_window_shows_through_its_glass_and_not_its_frame,
        layers::the_grid_has_the_spacing_chosen,
        layers::the_cube_hides_the_lines_behind_it,
        layers::the_fountain_goes_on_with_the_frames,
        layers::the_glass_blends_its_own_way_after_the_fountain,
        layers::a_context_given_back_has_its_layers_made_again,
        light::at_noon_the_top_is_lit,
        light::the_lit_side_follows_the_sun,
        light::a_lamp_lights_the_side_facing_it,
        light::a_shiny_ball_has_a_highlight,
        picking::the_pointer_over_a_cube_is_told_and_shown,
        picking::a_click_on_a_cube_tells_where,
        picking::the_ball_in_the_way_is_clicked_not_the_cube_behind,
        picking::dragging_over_a_cube_doesnt_click_it,
        relief::a_normal_map_shows_ridges_the_light_falls_on,
        relief::a_height_map_shifts_the_ridges_seen_at_a_slant,
        relief::a_low_detail_draws_fewer_pixels_and_no_relief,
        shaders::a_ball_without_code_of_its_own_stands_still,
        shaders::the_ripples_of_a_vertex_shader_move_the_outline,
        shaders::a_fragment_shader_colors_the_ball_with_the_time,
        spinning::the_cube_turns,
        textures::the_crate_has_its_image_from_a_file,
        textures::the_preloaded_metal_shows_at_once,
        textures::the_images_that_came_are_counted,
        textures::the_floor_is_tiled_with_pixels,
        textures::the_holes_in_the_fence_show_what_is_behind,
        thousands::the_cubes_are_drawn_and_seen_going_round,
    ];

    // A test of a known, not yet fixed bug goes to
    // `tests.extend(known_failures(browser_tests![...]))` (see README).

    run_suite(suite(), tests);
}
