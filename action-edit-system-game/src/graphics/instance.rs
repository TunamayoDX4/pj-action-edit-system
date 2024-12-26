pub struct Square {
    pos: [f32; 2], 
    size: [f32; 2], 
    rot_angle: f32, 
    color: [f32; 4], 
}

pub struct Polygon {
    point: [[f32; 2]; 3], 
    color: [f32; 4], 
}

pub struct Imaged {
    uv: [f32; 2], 
}