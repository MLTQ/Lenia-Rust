import ctypes
import numpy as np
from numpy.ctypeslib import ndpointer
from enum import Enum
import pygame
import pygame.time
import time
import pygame_gui
from pygame_gui.elements import UIButton, UILabel, UIHorizontalSlider

# Load the shared library
lib = ctypes.CDLL("./target/release/liblenia_3.dylib")

# Define the GrowthFuncType enum in Python
class GrowthFuncType(Enum):
    POLYNOMIAL = 0
    EXPONENTIAL = 1
    STEP = 2

# Define the run_lenia function signature
lib.run_lenia.argtypes = [
    ndpointer(ctypes.c_double, flags="C_CONTIGUOUS"),
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    ndpointer(ctypes.c_double, flags="C_CONTIGUOUS"),
    ctypes.c_double,
    ctypes.c_double,
    ctypes.c_double,
    ctypes.c_int,  # GrowthFuncType
    ndpointer(ctypes.c_double, flags="C_CONTIGUOUS"),
]

# Test the function
input_array = np.random.rand(256, 256)
output_array = np.empty_like(input_array)

kernel_size = 21
num_peaks = 2
betas = np.array([1, 5.0])
mu = 1
sigma = 0.15
dt = 0.19
growth_func_type = 1

# Initialize pygame
pygame.init()
scale_factor = 4
width, height = input_array.shape[1] * scale_factor, input_array.shape[0] * scale_factor
screen = pygame.display.set_mode((width, height))
pygame.display.set_caption("Lenia Simulation")

running = True

while running:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            running = False

    lib.run_lenia(input_array, input_array.shape[0], input_array.shape[1], kernel_size, num_peaks, betas, mu, sigma, dt, growth_func_type, output_array)

    input_array = output_array.copy()

    # Scale up the matrix by a factor of 2
    scaled_output_array = np.repeat(np.repeat(output_array, scale_factor, axis=0), scale_factor, axis=1)

    pixel_array = np.uint8(np.clip(scaled_output_array * 255, 0, 255)).repeat(3, -1).reshape(height, width, 3)
    surface = pygame.surfarray.make_surface(pixel_array)
    screen.blit(surface, (0, 0))

    pygame.display.flip()
    #check if matrix is empty, if so, quit:
    if np.count_nonzero(output_array) == 0:
        running = False
pygame.quit()
