# Amethyst_terrain

## What is this?
This is an expiremental renderpass for terrains in Amethyst using [Cardinal Neighbor Quadtrees][cnquadtree] and tesselation for Level-of-Detail.


[cnquadtree]: https://doi.org/10.5120/ijca2015907501

## Principles
A terrain heightmap is split based on the distance to the viewer using Cardinal Neighbor Quadtrees.
Each leaf is tesselated in regards to its neighbor to avoid T-Junctions.

After that a basic quad mesh is drawn using instaced attributes for each leaf.


## Future
Tries to have a similar good performance as major engine solutions.
First step is Unreal and Unity final step would be FarCry5s engine.


Some specific targets are:
* Asset streaming and thus spliting of the terrain assets (heightmap, etc.)
* Decals
* Integration into amethyst-atelier
* Fallback for lower spec systems without tesselation support

## Limitations
This approach uses tesselation and thus does currently not support metal or opengl < 4.1


## Building
Currently this [PR][https://github.com/amethyst/amethyst/pull/1866] is required to run this.
The crates here will try to load amethyst from the dir ../amethyst until the PR landed and 0.13 is released.


## Whats in here
* A [cnquadtree][cnquadtree_crate] crate implementing the algorithm from Safwan W. Qasem and Ameur A.
* The renderpass for Amethyst in the crate [amethyst_terrain][amethyst_terrain_crate]
* A simple demonstration crate implementing a [game][game_crate] with a simple terrain created with Quaea

[cnquadtree_crate]: cnquadtree
[amethyst_terrain_crate]: amethyst_terrain
[game_crate]: amethyst_terrain

## Inspiration
This approach was inspired by 
* https://github.com/drecuk/QuadtreeTerrain/blob/master/30.SYS-QuadtreeTerrain/TerrainQuadTree.cpp
* https://bitbucket.org/victorbush/ufl.cap5705.terrain/src/93c5ab3824a5a66d87d1bb6dcc9ed9aee7a16357/src_non_uniform/shader/?at=master
* https://developer.nvidia.com/gpugems/GPUGems2/gpugems2_chapter07.html 
* and the FarCry5 GDC Slides.