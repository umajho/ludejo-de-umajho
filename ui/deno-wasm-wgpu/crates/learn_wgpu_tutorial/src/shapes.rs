use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct Shape {
    // vertices: &'static [Vertex],
    indices: &'static [u16],

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl Shape {
    pub fn new(
        device: &wgpu::Device,
        vertices: &'static [Vertex],
        indices: &'static [u16],
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            // vertices,
            indices,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn num_indices(&self) -> u32 {
        self.indices.len() as u32
    }
}

pub struct Shapes {
    normal: Shape,
    challenge: Shape,
}

impl Shapes {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            normal: Shape::new(device, Self::normal().0, Self::normal().1),
            challenge: Shape::new(device, Self::challenge().0, Self::challenge().1),
        }
    }

    const fn normal() -> (&'static [Vertex], &'static [u16]) {
        let vertices = &[
            // 0: A
            Vertex {
                position: [-0.0868241, 0.49240386, 0.0],
                color: [0.5, 0.0, 0.5],
            },
            // 1: B
            Vertex {
                position: [-0.49513406, 0.06958647, 0.0],
                color: [0.5, 0.0, 0.5],
            },
            // 2: C
            Vertex {
                position: [-0.21918549, -0.44939706, 0.0],
                color: [0.5, 0.0, 0.5],
            },
            // 3: D
            Vertex {
                position: [0.35966998, -0.3473291, 0.0],
                color: [0.5, 0.0, 0.5],
            },
            // 4: E
            Vertex {
                position: [0.44147372, 0.2347359, 0.0],
                color: [0.5, 0.0, 0.5],
            },
        ];

        let indices = &[/**/ 0, 1, 4, /**/ 1, 2, 4, /**/ 2, 3, 4];

        (vertices, indices)
    }

    const fn challenge() -> (&'static [Vertex], &'static [u16]) {
        macro c {
            (LEAF) => {[0.0, 249.0 / 255.0, 0.0]},
            (TRUNK) => {[148.0 / 255.0, 82.0 / 255.0, 0.0]},
            (DECO) => {[1.0, 251.0 / 255.0, 0.0]},
            (AKANE) => {[1.0, (0x7e as f32)/255.0, (0x79 as f32)/255.0]},
            (AOI) => {[(0x76 as f32)/255.0, (0xd6 as f32)/255.0, 1.0]},
        }

        macro v($x:expr, $y:expr, $color:expr $(,)?) {
            Vertex {
                position: [$x as f32, $y as f32, 0.0],
                color: $color,
            }
        }

        const fn flip_y(v: &Vertex) -> Vertex {
            Vertex {
                position: [-v.position[0], v.position[1], v.position[2]],
                color: v.color,
            }
        }

        const fn between(v1: &Vertex, v2: &Vertex, fraction: f64) -> Vertex {
            Vertex {
                position: [
                    v1.position[0] + (v2.position[0] - v1.position[0]) * fraction as f32,
                    v1.position[1] + (v2.position[1] - v1.position[1]) * fraction as f32,
                    0.0,
                ],
                color: v1.color,
            }
        }

        const AMPLITUDE: f32 = 0.02;
        const fn sine_between_y_flipped(
            v: &Vertex,
            periods: f64,
            fraction: f64,
            color: [f32; 3],
        ) -> Vertex {
            Vertex {
                position: [
                    v.position[0] + 2.0 * -v.position[0] * fraction as f32,
                    v.position[1]
                        + (AMPLITUDE
                            * ::trig_const::sin(
                                periods as f64 * ::std::f64::consts::TAU * fraction as f64,
                            ) as f32),
                    0.0,
                ],
                color,
            }
        }

        const fn translate(v: &Vertex, dx: f64, dy: f64) -> Vertex {
            Vertex {
                position: [v.position[0] + dx as f32, v.position[1] + dy as f32, 0.0],
                color: v.color,
            }
        }

        const TOPPER_X: f64 = 0.0;
        const TOPPER_Y: f64 = 0.7;
        const TOPPER_OUTER_R: f64 = 0.05;
        const TOPPER_INNER_R: f64 = 0.03;

        macro topper {
            (outer, $n:literal) => {
                topper!(@circle, $n, TOPPER_OUTER_R)
            },
            (inner, $n:literal) => {
                topper!(@circle, $n as f64 + 0.5, TOPPER_INNER_R)
            },
            (@circle, $n:expr, $r:expr) => {
                v!(
                    TOPPER_X + $r * ::trig_const::sin($n as f64 * ::std::f64::consts::TAU / 5.0),
                    TOPPER_Y + $r * ::trig_const::cos($n as f64 * ::std::f64::consts::TAU / 5.0),
                    c!(DECO),
                )
            },
        }

        macro light {
            (a, $a:expr, $b:expr, $ab_fraction:expr, $periods:expr, $fraction:expr) => {
                sine_between_y_flipped(
                    &between(&$a, &$b, $ab_fraction),
                    $periods,
                    $fraction,
                    c!(DECO)
                )
            },
            (b, $a:ident) => {
                translate(&$a, -0.01, -0.02)
            },
            (c: $a:ident) => {
                translate(&$a, 0.01, -0.02)
            }
        }

        macro gs($([$a:literal,$b:literal,$c:literal]),* $(,)?) {
            &[$($a as u16, $b as u16, $c as u16),*]
        }

        static TOP_A: Vertex = v!(0, 0.7, c!(LEAF));
        static TOP_B: Vertex = v!(-0.2, 0.5, c!(LEAF));
        static MIDDLE_A: Vertex = v!(0, 0.6, c!(LEAF));
        static MIDDLE_B: Vertex = v!(-0.3, 0.3, c!(LEAF));
        static BOTTOM_A: Vertex = v!(0, 0.5, c!(LEAF));
        static BOTTOM_B: Vertex = v!(-0.4, 0.1, c!(LEAF));

        static TOP_LIGHT_1_A: Vertex = light!(a, TOP_A, TOP_B, 0.5, 3.0, 0.7 / 3.0);
        static TOP_LIGHT_2_A: Vertex = light!(a, TOP_A, TOP_B, 0.5, 4.0, 1.7 / 3.0);
        static TOP_LIGHT_3_A: Vertex = light!(a, TOP_A, TOP_B, 0.5, 3.0, 2.5 / 3.0);

        static MIDDLE_LIGHT_1_A: Vertex = light!(a, MIDDLE_A, MIDDLE_B, 0.65, 3.0, 0.7 / 4.0);
        static MIDDLE_LIGHT_2_A: Vertex = light!(a, MIDDLE_A, MIDDLE_B, 0.65, 3.0, 1.6 / 4.0);
        static MIDDLE_LIGHT_3_A: Vertex = light!(a, MIDDLE_A, MIDDLE_B, 0.65, 3.0, 2.5 / 4.0);
        static MIDDLE_LIGHT_4_A: Vertex = light!(a, MIDDLE_A, MIDDLE_B, 0.65, 3.0, 3.3 / 4.0);

        static BOTTON_LIGHT_1_A: Vertex = light!(a, BOTTOM_A, BOTTOM_B, 0.75, 3.0, 0.9 / 5.0);
        static BOTTON_LIGHT_2_A: Vertex = light!(a, BOTTOM_A, BOTTOM_B, 0.75, 3.0, 1.4 / 5.0);
        static BOTTON_LIGHT_3_A: Vertex = light!(a, BOTTOM_A, BOTTOM_B, 0.75, 3.0, 2.3 / 5.0);
        static BOTTON_LIGHT_4_A: Vertex = light!(a, BOTTOM_A, BOTTOM_B, 0.75, 3.0, 3.6 / 5.0);
        static BOTTON_LIGHT_5_A: Vertex = light!(a, BOTTOM_A, BOTTOM_B, 0.75, 3.0, 4.5 / 5.0);

        static AKANE_A: Vertex = v!(0.06, -0.06, c!(AKANE));
        static AOI_A: Vertex = v!(-0.4, -0.1, c!(AOI));

        static VERTICES: &'static [Vertex] = &[
            // tree leaves top
            TOP_A,
            TOP_B,
            flip_y(&TOP_B),
            // topper
            topper!(outer, 0), // 3: outer A
            topper!(inner, 0), // 4: inner AB
            topper!(outer, 1), // 5: outer B
            topper!(inner, 1), // 6: inner BC
            topper!(outer, 2), // 7: outer C
            topper!(inner, 2), // 8: inner CD
            topper!(outer, 3), // 9: outer D
            topper!(inner, 3), // 10: inner DE
            topper!(outer, 4), // 11: outer E
            topper!(inner, 4), // 12: inner EA
            // tree leaves middle
            MIDDLE_A,
            MIDDLE_B,
            flip_y(&MIDDLE_B),
            // tree leaves bottom
            BOTTOM_A,
            BOTTOM_B,
            flip_y(&BOTTOM_B),
            // tree trunk
            v!(-0.1, 0.1, c!(TRUNK)),
            v!(-0.1, -0.2, c!(TRUNK)),
            v!(0.1, -0.2, c!(TRUNK)),
            v!(0.1, 0.1, c!(TRUNK)),
            //
            v!(0, 0.7, c!(DECO)), // 23: topper center
            // top lights
            TOP_LIGHT_1_A,
            light!(b, TOP_LIGHT_1_A),
            light!(c: TOP_LIGHT_1_A),
            TOP_LIGHT_2_A,
            light!(b, TOP_LIGHT_2_A),
            light!(c: TOP_LIGHT_2_A),
            TOP_LIGHT_3_A,
            light!(b, TOP_LIGHT_3_A),
            light!(c: TOP_LIGHT_3_A),
            // middle lights
            MIDDLE_LIGHT_1_A,
            light!(b, MIDDLE_LIGHT_1_A),
            light!(c: MIDDLE_LIGHT_1_A),
            MIDDLE_LIGHT_2_A,
            light!(b, MIDDLE_LIGHT_2_A),
            light!(c: MIDDLE_LIGHT_2_A),
            MIDDLE_LIGHT_3_A,
            light!(b, MIDDLE_LIGHT_3_A),
            light!(c: MIDDLE_LIGHT_3_A),
            MIDDLE_LIGHT_4_A,
            light!(b, MIDDLE_LIGHT_4_A),
            light!(c: MIDDLE_LIGHT_4_A),
            // bottom lights
            BOTTON_LIGHT_1_A,
            light!(b, BOTTON_LIGHT_1_A),
            light!(c: BOTTON_LIGHT_1_A),
            BOTTON_LIGHT_2_A,
            light!(b, BOTTON_LIGHT_2_A),
            light!(c: BOTTON_LIGHT_2_A),
            BOTTON_LIGHT_3_A,
            light!(b, BOTTON_LIGHT_3_A),
            light!(c: BOTTON_LIGHT_3_A),
            BOTTON_LIGHT_4_A,
            light!(b, BOTTON_LIGHT_4_A),
            light!(c: BOTTON_LIGHT_4_A),
            BOTTON_LIGHT_5_A,
            light!(b, BOTTON_LIGHT_5_A),
            light!(c: BOTTON_LIGHT_5_A),
            //
            AKANE_A,
            translate(&AKANE_A, 0.0, -0.64),
            translate(&AKANE_A, 0.24, -0.64),
            translate(&AKANE_A, 0.24, 0.0),
            AOI_A,
            translate(&AOI_A, 0.0, -0.64),
            translate(&AOI_A, 0.24, -0.64),
            translate(&AOI_A, 0.24, 0.0),
        ];

        static INDICES: &'static [u16] = gs!(
            [0, 1, 2], // tree leaves top
            // topper
            [3, 23, 4],
            [4, 23, 5],
            [5, 23, 6],
            [6, 23, 7],
            [7, 23, 8],
            [8, 23, 9],
            [9, 23, 10],
            [10, 23, 11],
            [11, 23, 12],
            [12, 23, 3],
            //
            [13, 14, 15], // tree leaves middle
            [16, 17, 18], // tree leaves bottom
            // tree trunk
            [19, 20, 21],
            [19, 21, 22],
            // top lights
            [24, 25, 26],
            [27, 28, 29],
            [30, 31, 32],
            // middle lights
            [33, 34, 35],
            [36, 37, 38],
            [39, 40, 41],
            [42, 43, 44],
            // bottom lights
            [45, 46, 47],
            [48, 49, 50],
            [51, 52, 53],
            [54, 55, 56],
            [57, 58, 59],
            //
            [60, 61, 62],
            [60, 62, 63],
            [64, 65, 66],
            [64, 66, 67],
        );

        (VERTICES, INDICES)
    }

    pub fn get(&self, is_challenge: bool) -> &Shape {
        if is_challenge {
            &self.challenge
        } else {
            &self.normal
        }
    }
}
