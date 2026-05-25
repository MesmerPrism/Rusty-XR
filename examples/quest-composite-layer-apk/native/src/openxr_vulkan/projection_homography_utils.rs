pub(super) fn identity_homography() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

pub(super) fn full_target_canvas_clip() -> [[f32; 4]; 4] {
    [
        [-1.0, -1.0, 0.0, 1.0],
        [1.0, -1.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
        [-1.0, 1.0, 0.0, 1.0],
    ]
}

pub(super) fn pack_homography_row(row: [f32; 3]) -> [f32; 4] {
    [row[0], row[1], row[2], 0.0]
}

pub(super) fn screen_to_domain_with_visual_offset(
    mut rows: [[f32; 3]; 3],
    offset_x_uv: f32,
    offset_y_uv: f32,
) -> [[f32; 3]; 3] {
    let input_x_offset = -offset_x_uv.clamp(-0.5, 0.5);
    let input_y_offset = -offset_y_uv.clamp(-0.5, 0.5);
    for row in &mut rows {
        row[2] += row[0] * input_x_offset + row[1] * input_y_offset;
    }
    rows
}

pub(super) fn domain_to_screen_with_visual_offset(
    mut rows: [[f32; 3]; 3],
    offset_x_uv: f32,
    offset_y_uv: f32,
) -> [[f32; 3]; 3] {
    let output_x_offset = offset_x_uv.clamp(-0.5, 0.5);
    let output_y_offset = offset_y_uv.clamp(-0.5, 0.5);
    let projective_row = rows[2];
    for (column, projective_value) in projective_row.into_iter().enumerate() {
        rows[0][column] += projective_value * output_x_offset;
        rows[1][column] += projective_value * output_y_offset;
    }
    rows
}
