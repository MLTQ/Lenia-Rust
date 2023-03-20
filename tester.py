import ctypes
import numpy as np
from numpy.ctypeslib import ndpointer
from enum import Enum
import pygame
import pygame.time
import random
import time
import pygame_gui
from pygame_gui.elements import UIButton, UILabel, UIHorizontalSlider

# Define the GrowthFuncType enum, honestly this is just a reminder for me
class GrowthFuncType(Enum):
    POLYNOMIAL = 0
    EXPONENTIAL = 1
    STEP = 2


# Load the dylib... this should be broken out somewhere or I should learn to make better rust libraries!
lib = ctypes.CDLL("./target/release/liblenia_3.dylib")
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

def generate_gaussian_blob(size, amplitude=0.1, sigma=0.1, mu=1.0):
    x, y = np.meshgrid(np.linspace(-1, 1, size), np.linspace(-1, 1, size))
    distance = np.sqrt(x * x + y * y)
    #create a gaussian in the center of the grid
    gaussian = amplitude * np.exp(-((distance - mu) ** 2 / (2.0 * sigma ** 2)))
    #gaussian = amplitude * np.exp(-distance ** 2)
    return gaussian

M = N = 256
kernel_size=21
num_peaks=2
betas=np.array([1, 5.0])
mu=1.0
sigma=0.14
dt=0.21
growth_func_type=1
scale_factor = 3
food = True
P = 10 # Refresh food sources every P frames
F = 10  # Number of food sources
blob_size = 21  # Size of the Gaussian blob
# Inside the main loop
frame_counter = 0
input_array = np.random.rand(M, N)
output_array = np.empty_like(input_array)

if food:
    randomize_food = False
    food_source_positions = []
    for _ in range(F):
        i = random.randint(0, M - blob_size)
        j = random.randint(0, N - blob_size)
        input_array[i:i + blob_size, j:j + blob_size] += generate_gaussian_blob(blob_size)
        if not randomize_food:
            food_source_positions.append((i, j))

# Initialize pygame
pygame.init()

width, height = input_array.shape[1] * scale_factor, input_array.shape[0] * scale_factor
screen = pygame.display.set_mode((width, height))
pygame.display.set_caption("Lenia Simulation")

running = True
while running:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            running = False
    frame_counter += 1

    lib.run_lenia(input_array, input_array.shape[0], input_array.shape[1], kernel_size, num_peaks, betas, mu, sigma, dt, growth_func_type, output_array)
    input_array = output_array.copy()

    # Scale up the matrix to make things nicer towatch
    scaled_output_array = np.repeat(np.repeat(output_array, scale_factor, axis=0), scale_factor, axis=1)

    # Refresh food sources
    if frame_counter % P == 0:
        for _ in range(F):
            if randomize_food:
                i = random.randint(0, M - blob_size)
                j = random.randint(0, N - blob_size)
                input_array[i:i + blob_size, j:j + blob_size] += generate_gaussian_blob(blob_size)
            else:
                for i, j in food_source_positions:
                    input_array[i:i + blob_size, j:j + blob_size] += generate_gaussian_blob(blob_size)

    pixel_array = np.uint8(np.clip(scaled_output_array * 255, 0, 255)).repeat(3, -1).reshape(height, width, 3)
    surface = pygame.surfarray.make_surface(pixel_array)
    screen.blit(surface, (0, 0))

    pygame.display.flip()
    #Check if matrix is empty, if so, quit:
    if np.count_nonzero(output_array) == 0:
        running = False
pygame.quit()
