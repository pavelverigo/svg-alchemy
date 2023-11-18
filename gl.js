const vertexShaderSource = `#version 300 es

in vec2 a_position;
in vec4 a_color;
out vec4 v_color;
uniform vec2 u_resolution;

void main() {
    vec2 clipSpace = (a_position / u_resolution) * 2.0 - 1.0;
    // rotate for left-top corner
    gl_Position = vec4(clipSpace * vec2(1, -1), 0, 1);

    v_color = a_color;
}
`;
 
const fragmentShaderSource = `#version 300 es
precision highp float;
 
in vec4 v_color;
out vec4 outColor;

void main() {
  outColor = v_color;
}
`;

function createShader(gl, type, source) {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    const success = gl.getShaderParameter(shader, gl.COMPILE_STATUS);
    if (success) {
        return shader;
    }

    console.log(gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
}

function createProgram(gl, vertexShader, fragmentShader) {
    const program = gl.createProgram();
    gl.attachShader(program, vertexShader);
    gl.attachShader(program, fragmentShader);
    gl.linkProgram(program);
    const success = gl.getProgramParameter(program, gl.LINK_STATUS);
    if (success) {
        return program;
    }
   
    console.log(gl.getProgramInfoLog(program));
    gl.deleteProgram(program);
}

export function gl_init(canvas) {
    const gl = canvas.getContext("webgl2", {
        antialias: true,
        depth: false,

        alpha: false,
    });

    const vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource);
    const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource);

    const program = createProgram(gl, vertexShader, fragmentShader);

    const a_position = gl.getAttribLocation(program, 'a_position');
    const a_color = gl.getAttribLocation(program, 'a_color');

    const resolutionUniformLocation = gl.getUniformLocation(program, "u_resolution");

    const vertexBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);

    const vao = gl.createVertexArray();

    gl.bindVertexArray(vao);

    gl.vertexAttribPointer(a_position, 2, gl.FLOAT, false, 24, 0);
    gl.enableVertexAttribArray(a_position);

    gl.vertexAttribPointer(a_color, 4, gl.FLOAT, false, 24, 8);
    gl.enableVertexAttribArray(a_color);

    const indexBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);

    gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.useProgram(program);
    gl.uniform2f(resolutionUniformLocation, gl.canvas.width, gl.canvas.height);

    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    return gl;
}

export function gl_set_clear_color(gl, r, g, b, a) {
    gl.clearColor(r, g, b, a);
}

export function gl_draw_triangles(gl, vertices, indices) {
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.DYNAMIC_DRAW, 0);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.DYNAMIC_DRAW, 0);
    gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);
}

export function gl_draw_wireframe(gl, vertices, indices) {
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.DYNAMIC_DRAW, 0);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.DYNAMIC_DRAW, 0);
    gl.drawElements(gl.LINES, indices.length, gl.UNSIGNED_INT, 0);
}