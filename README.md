This is an implementation of Lenia (continuous cellular automata) in Rust.

## Frontends

### Native egui frontend (new)

```bash
cargo run
```

What you can do in the egui app:
- Play/pause the simulation, single-step, clear, randomize
- Tune Lenia parameters: `kernel_size`, `num_peaks`, `betas`, `mu`, `sigma`, `dt`, `growth_func_type`
- Enable periodic food placement (fixed or randomized food sources)
- Draw directly on the grid with:
  - `Draw Life`
  - `Erase`
  - `Place Food`

### Python frontend (legacy)

Build the release library, then run:

```bash
cargo build --release
python tester.py
```

Here's a brief explanation of each parameter and its role in the Lenia simulation:

kernel_size: This parameter defines the size of the interaction kernel, which determines the neighborhood around each cell that is considered when calculating the next state of the cell. A larger kernel size means that cells 
can "sense" and interact with a wider range of neighboring cells, while a smaller kernel size results in more local interactions.

num_peaks: This parameter specifies the number of peaks in the kernel function. Multiple peaks can result in more complex interaction patterns between cells, as they introduce different interaction strengths at different 
distances.

betas: This is an array containing the beta values used to define the shape of the kernel function. Higher beta values result in a more "peaked" function, meaning that interactions between cells are more focused around the 
peaks, while lower beta values produce a smoother function, leading to more diffuse interactions.

mu: This is the life parameter, which controls the balance between the growth and decay of cell states. Higher values of mu favor growth, while lower values favor decay. Adjusting mu can change the overall dynamics of the 
system and determine whether patterns emerge or fade away.

sigma: This is the standard deviation of the Gaussian function used in the convolution process. A smaller sigma results in a more focused interaction between cells, while a larger sigma produces a more diffuse interaction 
pattern.

dt: This is the time step of the simulation, which controls the speed of the simulation's update process. Smaller values of dt result in slower, smoother dynamics, while larger values can lead to more rapid and potentially 
chaotic behavior.

growth_func_type: This parameter determines the type of growth function used in the Lenia algorithm. Different growth functions can produce different emergent behaviors in the system. The available options are POLYNOMIAL, 
EXPONENTIAL, and STEP.

Keep in mind that these parameters can interact with each other in complex ways, so finding the right combination to produce desired behaviors might require some experimentation and fine-tuning.





