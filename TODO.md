next steps:

- find more textured test material
  - verify if there is any distortion
- clone texture coordinates to match cloned vertex coordinates (?)
  - perhaps creating an intermediate Vertex object that includes all the remapped indices would be easier?
- unbind VAOs, etc., when doing rendering/binding
- respect normals specified in .obj file
- implement normal mapping
- implement debug mode (render normals, etc.)
- implement smoothing groups from .obj file (https://www.opengl.org/discussion_boards/showthread.php/185705-How-do-I-use-smoothing-groups-from-obj-files?s=82789c08fa5766b6f65b4a47349c4bc7&p=1264445&viewfull=1#post1264445 and https://www.opengl.org/discussion_boards/showthread.php/185705-How-do-I-use-smoothing-groups-from-obj-files?p=1264602&viewfull=1#post1264602)
