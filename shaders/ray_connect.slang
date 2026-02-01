// ray_connect.slang
//
// Performs ray connection for all samples waiting in the connection queue.
// Ray connection involves intersecting a ray with any element in
// the scene. This is a useful distinction from extend, as shadow
// rays only need to identify an occlusion, not the nearest occlusion.
module ray_connect;
