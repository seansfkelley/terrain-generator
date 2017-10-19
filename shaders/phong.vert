#version 410

uniform mat4 mvp;

in vec3 in_Position;
in vec3 in_ColorAmbient;
in vec3 in_ColorDiffuse;

out vec3 out_ColorAmbient;
out vec3 out_ColorDiffuse;

void main() {
    gl_Position = mvp * vec4(in_Position, 1.0);
    out_ColorAmbient = in_ColorAmbient;
    out_ColorDiffuse = in_ColorDiffuse;
}
