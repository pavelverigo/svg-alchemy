use tiny_skia_path::{PathSegment, PathSegmentsIter};

use tiny_skia_path:: Point as skia_Point;
use lyon::math::Point as lyon_Point;
use lyon::path::PathEvent;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{self, FillOptions, FillTessellator, StrokeOptions, StrokeTessellator};
use usvg::*;

use js_sys::{Reflect, Object};

use wasm_bindgen::prelude::*;

use log::Level;

pub const FALLBACK_COLOR: usvg::Color = usvg::Color {
    red: 0,
    green: 0,
    blue: 0,
};

#[wasm_bindgen]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[wasm_bindgen(start)]
pub fn start() {
    console_log::init_with_level(Level::Debug).unwrap();
}

#[wasm_bindgen]
pub struct TessellateOut {
    mesh: VertexBuffers<MyVertex, u32>,
    view_box: NonZeroRect,
    size: Size,
}

#[wasm_bindgen]
impl TessellateOut {
    pub fn vertices(&self) -> Vec<f32> {
        self.mesh.vertices.iter().map(|v| v.pos.to_vec()).flatten().collect()
    }

    pub fn colors(&self) -> Vec<u32> {
        self.mesh.vertices.iter().map(|v| {
            let r = u32::from(v.color.red);
            let g = u32::from(v.color.green);
            let b = u32::from(v.color.blue);
            let a: u32 = 255;
            (r << 24) | (g << 16) | (b << 8) | a
        }).collect()
    }

    pub fn indices(&self) -> Vec<u32> {
        self.mesh.indices.clone()
    }

    pub fn view_box(&self) -> JsValue  {
        let js_obj = Object::new();
        Reflect::set(&js_obj, &JsValue::from_str("left"), &JsValue::from(self.view_box.left())).unwrap();
        Reflect::set(&js_obj, &JsValue::from_str("top"), &JsValue::from(self.view_box.top())).unwrap();
        Reflect::set(&js_obj, &JsValue::from_str("right"), &JsValue::from(self.view_box.right())).unwrap();
        Reflect::set(&js_obj, &JsValue::from_str("bottom"), &JsValue::from(self.view_box.bottom())).unwrap();
        JsValue::from(js_obj)
    }

    pub fn size_width(&self) -> f32 {
        self.size.width()
    }

    pub fn size_height(&self) -> f32 {
        self.size.height()
    }
}


#[wasm_bindgen]
pub fn tessellate_svg(svg_data: Vec<u8>) -> TessellateOut {
    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let mut mesh: VertexBuffers<MyVertex, u32> = VertexBuffers::new();

    let opt = usvg::Options::default();
    let rtree = usvg::Tree::from_data(&svg_data, &opt).unwrap();

    let tolerance = StrokeOptions::DEFAULT_TOLERANCE;
    
    let svg_view_box = rtree.view_box.rect;
    let svg_size = rtree.size;

    for node in rtree.root.descendants() {
        match *node.borrow() {
            usvg::NodeKind::Group(_) => (),
            usvg::NodeKind::Image(_) => (),
            usvg::NodeKind::Text(_) => (),
            usvg::NodeKind::Path(ref p) => {
                if let Some(ref fill) = p.fill {
                    // fall back to always use color fill
                    // no gradients (yet?)
                    let color = match fill.paint {
                        usvg::Paint::Color(c) => c,
                        _ => FALLBACK_COLOR,
                    };
    
                    fill_tess
                        .tessellate(
                            PathConvIter::new(p),
                                &FillOptions::tolerance(tolerance),
                                &mut BuffersBuilder::new(
                                    &mut mesh,
                                    VertexConstructor {
                                        transform: node.abs_transform(),
                                        color,
                                    },
                                ),
                        )
                        .expect("Error during tessellation!");
                }
    
                if let Some(ref stroke) = p.stroke {
                    let (stroke_color, stroke_opts) = convert_stroke(stroke, tolerance);
                    let _ = stroke_tess.tessellate(
                        PathConvIter::new(p),
                            &stroke_opts.with_tolerance(tolerance),
                            &mut BuffersBuilder::new(
                                &mut mesh,
                                VertexConstructor {
                                    transform: node.abs_transform(),
                                    color: stroke_color,
                                },
                            ),
                    );
                }
            }
        }
    }

    log::debug!(
        "Finished tessellation: {} vertices, {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );

    TessellateOut { mesh, view_box: svg_view_box, size: svg_size }
}

struct PathConvIter<'a> {
    iter: PathSegmentsIter<'a>,
    prev: lyon_Point,
    first: lyon_Point,
    needs_end: bool,
    deferred: Option<PathEvent>,
}

impl<'a> PathConvIter<'a> {
    fn new(p: &'a usvg::Path) -> Self {
        Self {
            iter: p.data.segments(),
            first: lyon_Point::new(0.0, 0.0),
            prev: lyon_Point::new(0.0, 0.0),
            deferred: None,
            needs_end: false,
        }
    }
}

impl<'l> Iterator for PathConvIter<'l> {
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        if self.deferred.is_some() {
            return self.deferred.take();
        }

        let next = self.iter.next();
        match next {
            Some(PathSegment::MoveTo(skia_Point { x, y })) =>  {
                if self.needs_end {
                    let last = self.prev;
                    let first = self.first;
                    self.needs_end = false;
                    self.prev = lyon_Point::new(x, y);
                    self.deferred = Some(PathEvent::Begin { at: self.prev });
                    self.first = self.prev;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    self.first = lyon_Point::new(x, y);
                    self.needs_end = true;
                    Some(PathEvent::Begin { at: self.first })
                }
            }
            Some(PathSegment::LineTo(skia_Point { x, y })) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = lyon_Point::new(x, y);
                Some(PathEvent::Line {
                    from,
                    to: self.prev,
                })
            }
            Some(PathSegment::CubicTo(
                skia_Point { x: x1, y: y1 },
                skia_Point { x: x2, y: y2 },
                skia_Point { x, y },
            )) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = lyon_Point::new(x, y);
                Some(PathEvent::Cubic {
                    from,
                    ctrl1: lyon_Point::new(x1, y1),
                    ctrl2: lyon_Point::new(x2, y2),
                    to: self.prev,
                })
            }
            Some(PathSegment::QuadTo(
                skia_Point { x: x1, y: y1 },
                skia_Point { x, y },
            )) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = lyon_Point::new(x, y);
                Some(PathEvent::Quadratic {
                    from,
                    ctrl: lyon_Point::new(x1, y1),
                    to: self.prev,
                })
            }
            Some(PathSegment::Close) => {
                self.needs_end = false;
                self.prev = self.first;
                Some(PathEvent::End {
                    last: self.prev,
                    first: self.first,
                    close: true,
                })
            }
            None => {
                if self.needs_end {
                    self.needs_end = false;
                    let last = self.prev;
                    let first = self.first;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    None
                }
            }
        }
    }
}

fn convert_stroke(s: &usvg::Stroke, tolerance: f32) -> (usvg::Color, StrokeOptions) {
    let color = match s.paint {
        usvg::Paint::Color(c) => c,
        _ => FALLBACK_COLOR,
    };
    let linecap = match s.linecap {
        usvg::LineCap::Butt => tessellation::LineCap::Butt,
        usvg::LineCap::Square => tessellation::LineCap::Square,
        usvg::LineCap::Round => tessellation::LineCap::Round,
    };
    let linejoin = match s.linejoin {
        usvg::LineJoin::Miter => tessellation::LineJoin::Miter,
        usvg::LineJoin::Bevel => tessellation::LineJoin::Bevel,
        usvg::LineJoin::Round => tessellation::LineJoin::Round,
        usvg::LineJoin::MiterClip => tessellation::LineJoin::MiterClip,
    };

    let opt = StrokeOptions::tolerance(tolerance)
        .with_line_width(s.width.get())
        .with_line_cap(linecap)
        .with_line_join(linejoin);

    (color, opt)
}

struct MyVertex {
    pos: [f32; 2],
    color: usvg::Color,
}

struct VertexConstructor {
    transform: usvg::Transform,
    color: usvg::Color,
}

impl FillVertexConstructor<MyVertex> for VertexConstructor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> MyVertex {
        let pos = transform(&self.transform, vertex.position().to_array());
        MyVertex {
            pos,
            color: self.color,
        }
    }
}

impl StrokeVertexConstructor<MyVertex> for VertexConstructor {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> MyVertex {
        let pos = transform(&self.transform, vertex.position().to_array());
        MyVertex {
            pos,
            color: self.color,
        }
    }
}

fn transform(t: &usvg::Transform, [x, y]: [f32; 2]) -> [f32; 2] {
    let mut point = tiny_skia_path::Point { x, y };
    t.map_point(&mut point);
    [point.x, point.y]
}

#[wasm_bindgen]
pub fn simplify_svg(svg_data: Vec<u8>) -> Vec<u8> {
    let svg_opt = usvg::Options::default();
    let tree = usvg_tree::Tree::from_data(&svg_data, &svg_opt).unwrap();
    let xml_opt = usvg::XmlOptions::default();

    tree.to_string(&xml_opt).into_bytes()
}