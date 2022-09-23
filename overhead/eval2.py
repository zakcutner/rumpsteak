#!/usr/bin/env python


import argparse
import json
from unittest import result
import matplotlib.pyplot as plt
import numpy as np
import statistics

import pandas as pd
from scipy import stats

def moving_average(times, num_runs):
    times_padded = np.pad(
        times, (num_runs // 2, num_runs - 1 - num_runs // 2), mode="edge"
    )
    kernel = np.ones(num_runs) / num_runs
    return np.convolve(times_padded, kernel, mode="valid")


import numpy as np


def average(data):
    return sum(data) / len(data)

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("file", help="JSON file with benchmark results")
parser.add_argument("--title", help="Plot Title")
parser.add_argument("-o", "--output", help="Save image to the given filename.")


args = parser.parse_args()

with open(args.file) as f:
    results = json.load(f)["results"]

mean = results[0]["mean"]

arr = results[0]["times"]
arr.sort()


interval=stats.norm.interval(0.95,statistics.mean(arr),statistics.stdev(arr))

print("******mean******")
print(mean)

print("******confidence interval******")
print(interval)
