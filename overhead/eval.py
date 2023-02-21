#!/usr/bin/env python

"""This program shows `hyperfine` benchmark results in a sequential way
in order to debug possible background interference, caching effects,
thermal throttling and similar effects.
"""

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


def median(x):
    x = sorted(x)
    length = len(x)
    mid, rem = divmod(length, 2)    # divmod函数返回商和余数
    if rem:
        return x[:mid], x[mid+1:], x[mid]
    else:
        return x[:mid], x[mid:], (x[mid-1]+x[mid])/2
 

import numpy as np


def average(data):
    return sum(data) / len(data)


def bootstrap(data, B, c, func):
    """
    计算bootstrap置信区间
    :param data: array 保存样本数据
    :param B: 抽样次数 通常B>=1000
    :param c: 置信水平
    :param func: 样本估计量
    :return: bootstrap置信区间上下限
    """
    array = np.array(data)
    n = len(array)
    sample_result_arr = []
    for i in range(B):
        index_arr = np.random.randint(0, n, size=n)
        data_sample = array[index_arr]
        sample_result = func(data_sample)
        sample_result_arr.append(sample_result)

    a = 1 - c
    k1 = int(B * a / 2)
    k2 = int(B * (1 - a / 2))
    auc_sample_arr_sorted = sorted(sample_result_arr)
    lower = auc_sample_arr_sorted[k1]
    higher = auc_sample_arr_sorted[k2]

    return lower, higher


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("file1", help="JSON file with benchmark results")
parser.add_argument("file2", help="JSON file with benchmark results")
parser.add_argument("--title", help="Plot Title")
parser.add_argument("-o", "--output", help="Save image to the given filename.")


args = parser.parse_args()

with open(args.file1) as f:
    results = json.load(f)["results"]

mean1 = results[0]["mean"]

arr1 = results[0]["times"]
arr1.sort()
lHalf, rHalf, q2 = median(arr1)

with open(args.file2) as f:
    results = json.load(f)["results"]
mean2 = results[0]["mean"]
arr2 = results[0]["times"]
arr2.sort()

arr = [arr2[i]-arr1[i] for i in range(0,len(arr1))]

lHalf, rHalf, q2 = median(arr)
print("*****medium*******")
print(median(lHalf)[2])
print(q2)
print(median(rHalf)[2])
print("******bootstrap******")

result = bootstrap(arr, 1000, 0.95, average)
print(result)

# label = results[0]["command"]
# times = results[0]["times"]
# num = len(times)
# nums = range(num)


# prob=stats.norm.pdf(0,mean,std) #在0处概率密度值
# pre=stats.norm.cdf(0,mean,std)  #预测小于0的概率
interval=stats.norm.interval(0.95,statistics.mean(arr),statistics.stdev(arr))

print("******confidence interval******")
print(interval)
diff =mean2-mean1
# fi = open('output.txt', 'w')

print("difference:", diff)
print('percent:{:.2%}'.format(diff/mean1))

# print 'difference', diff
# fi.close()