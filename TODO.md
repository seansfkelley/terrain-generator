next steps:

- clone texture coordinates to match cloned vertex coordinates
  - perhaps creating an intermediate Vertex object that includes all the remapped indices would be easier?
- figure out why texture on cube is distorted
- RenderableObject should do all the loading/parsing on its own (i.e. give it the path)
  - this seems like a simpler API and doesn't require us to uselessly hold onto objects in memory once they're pushed to the GPU
- figure out how to escape the texture ID from the chunking so it can be used during rendering
- figure out what the meaning of glActiveTexture/TEXTURE0 actually is and how to use it
- RenderableChunk should maybe be 1:1 with VAO so that it's easier to swap textures?
- unbind VAOs when doing rendering/binding
- respect normals specified in .obj file
- implement normal mapping
- implement debug mode (render normals, etc.)
- implement smoothing groups from .obj file (https://www.opengl.org/discussion_boards/showthread.php/185705-How-do-I-use-smoothing-groups-from-obj-files?s=82789c08fa5766b6f65b4a47349c4bc7&p=1264445&viewfull=1#post1264445 and https://www.opengl.org/discussion_boards/showthread.php/185705-How-do-I-use-smoothing-groups-from-obj-files?p=1264602&viewfull=1#post1264602)
