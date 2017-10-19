#version 410

uniform mat4 u_MatMvp;
uniform mat4 u_MatV;
uniform mat4 u_MatM;
uniform vec3 u_LightPosition_WorldSpace;

in vec3 in_VertexPosition;
in vec3 in_VertexNormal;
in vec3 in_ColorAmbient;
in vec3 in_ColorDiffuse;
in vec3 in_ColorSpecular;
in float in_SpecularExponent;

out vec3 out_ColorAmbient;
out vec3 out_ColorDiffuse;
out vec3 out_ColorSpecular;
out float out_SpecularExponent;
out vec3 out_VertexPosition_WorldSpace;
out vec3 out_EyeDirection_CameraSpace;
out vec3 out_LightDirection_CameraSpace;
out vec3 out_VertexNormal_CameraSpace;

void main() {
    gl_Position = u_MatMvp * vec4(in_VertexPosition, 1.0);

    out_VertexPosition_WorldSpace = (u_MatM * vec4(in_VertexPosition, 1.0)).xyz;

    vec3 VertexPosition_CameraSpace = (u_MatV * u_MatM * vec4(in_VertexPosition, 1)).xyz;
	  out_EyeDirection_CameraSpace = vec3(0, 0, 0) - VertexPosition_CameraSpace;

    vec3 LightPosition_CameraSpace = (u_MatV* vec4(u_LightPosition_WorldSpace, 1)).xyz;
	  out_LightDirection_CameraSpace = LightPosition_CameraSpace + out_EyeDirection_CameraSpace;

    // N.B.: Not correct if scaling is in use.
    out_VertexNormal_CameraSpace = (u_MatV * u_MatM * vec4(in_VertexNormal, 0)).xyz;

    out_ColorAmbient = in_ColorAmbient;
    out_ColorDiffuse = in_ColorDiffuse;
    out_ColorSpecular = in_ColorSpecular;
    out_SpecularExponent = in_SpecularExponent;
}
