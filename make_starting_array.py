import numpy as np
from fire import Fire

# Generate a 512x512 array of random values between 0 and 1
def main(n):
   random_array = np.random.rand(512, 512)

   # Save the array as a CSV file
   np.savetxt("initial_state_large.csv", random_array, delimiter=",")

if __name__=='__main__':
   Fire(main)
