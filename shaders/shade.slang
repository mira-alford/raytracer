// shade.slang
//
// Performs shading of samples waiting in the shade queue.
// Shading entails computing the changes to sample state by
// using the most recent hit record.
// Shading also determines ray termination or bouncing,
// and will enqueue the sample for another extension accordingly.
module shade;
