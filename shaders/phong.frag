#version 410

in vec3 out_ColorAmbient;
in vec3 out_ColorDiffuse;

out vec3 color;

void main() {
    color = out_ColorAmbient + out_ColorDiffuse;
}
