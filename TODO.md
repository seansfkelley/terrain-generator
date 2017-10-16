next steps:

- switch to using glDrawElements instead of glDrawArrays (http://www.opengl-tutorial.org/intermediate-tutorials/tutorial-9-vbo-indexing/)
  - probably want some kind of obj -> automagically-binding-into-OpenGL-buffers function that then returns the names of the buffer(s)
- render ashuttle shape (http://people.sc.fsu.edu/%7Ejburkardt/data/obj/obj.html)
  - this requires being able to render quads too
- figure out how to use materials from the .obj for the shuttle
- implement basic shading for point light sources(http://www.opengl-tutorial.org/beginners-tutorials/tutorial-8-basic-shading/)
