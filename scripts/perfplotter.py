import csv
import os
import matplotlib
import matplotlib.pyplot as plt
from matplotlib.font_manager import FontProperties
from os.path import isfile, join
import itertools
import math

matplotlib.rcParams.update({'font.size': 10})

csvreader_imageproc  = csv.DictReader(open('imageproc_perfdata.csv'))
csvreader_mandelbrot = csv.DictReader(open('mandelbrot_perfdata.csv'))

def graph(xy, title, ylabel, xlabel, legend, file_path):
    print("Creating graph")
    
    fig, axs = plt.subplots(1,1)
    plt.title(title)

    x, y = xy
    line = axs.plot(
        x, y, legend
    )

    axs.set_ylabel(ylabel)
    axs.set_xlabel(xlabel)

    labels = [if0thenseq(int(item)) for item in axs.get_xticks()]
    
    axs.set_xticklabels(labels)   
 
    plt.savefig(file_path+'.pdf', bbox_inches="tight", format="pdf", dpi=200)

def get_xy(x_col, y_col, reader):
    x = []
    y = []

    for row in reader:
        x.append(float(row[x_col]))
        y.append(float(row[y_col]))
    return (x, y)

def if0thenseq(val, seq):
    if val == 0:
        return seq
    else:
        return val


def graph_multi(lines, title, ylabel, xlabel,file_path, seq_label="Sequential"):
    print("Creating graph")
    
    fig, axs = plt.subplots(1,1)
    
    ticks = list(filter(lambda x: x % 2 == 0, lines[0][0]))
    
    for (x,y,legend, marker, linestyle) in lines:
        line = axs.plot(
            x, y, 
            marker = marker, 
            label = legend, 
            linestyle = linestyle, 
            linewidth = 1,
            markersize = 3
        )

    axs.legend(loc='upper center', ncol = 3, bbox_to_anchor=(0.5, 1.25))

    axs.set_ylabel(ylabel)
    axs.set_xlabel(xlabel)
    
    plt.yticks(ticks)
    plt.xticks(ticks)
    
    plt.grid()

    labels = [if0thenseq(int(item), seq_label) for item in axs.get_xticks()]
    
    axs.set_xticklabels(labels) 
    axs.set_aspect(0.5)
    plt.savefig(file_path+'.pdf', bbox_inches="tight", format="pdf", dpi=300)


def graph_separated(lines, title, ylabel,file_path, seq_label="Sequential"):
    print("Creating graph")
    
    fig, axs = plt.subplots(2,1)
    
    ticks = list(filter(lambda x: x % 2 == 0, lines[0][0]))
    
    curplot = 1
    for (x,y,legend, marker, linestyle, xlabel) in lines:
        axs = plt.subplot(2,1,curplot)
        line = axs.plot(
            x, y, 
            marker = marker, 
            label = legend, 
            linestyle = linestyle, 
            linewidth = 1,
            markersize = 3
        )

        axs.legend(loc='upper center', ncol = 3, bbox_to_anchor=(0.5, 1.3))

        axs.set_ylabel(ylabel)
        axs.set_xlabel(xlabel)
        
        plt.yticks(ticks)
        plt.xticks(ticks)
        
        plt.grid()

        labels = [if0thenseq(int(item), seq_label) for item in axs.get_xticks()]
        
        axs.set_xticklabels(labels) 
        axs.set_aspect(0.5)

        curplot = curplot + 1
    plt.tight_layout()
    plt.savefig(file_path+'.pdf', bbox_inches="tight", format="pdf", dpi=300)



def plot_mandelbrot_comparison():
    x_rustspp, y_rustspp = get_xy("Level of Parallelism", "Speedup Rust-SPP", csv.DictReader(open('mandelbrot_perfdata.csv')))
    x_rustspp_ordered, y_rustspp_ordered = get_xy("Level of Parallelism", "Speedup Rust-SPP Ordered", csv.DictReader(open('mandelbrot_perfdata.csv')))
    x_rayon, y_rayon = get_xy("Level of Parallelism", "Speedup Rayon", csv.DictReader(open('mandelbrot_perfdata.csv')))
    x_tokio_ordered, y_tokio_ordered = get_xy("Level of Parallelism", "Speedup Tokio Ordered", csv.DictReader(open('mandelbrot_perfdata.csv')))
    x_tokio_unordered, y_tokio_unordered = get_xy("Level of Parallelism", "Speedup Tokio Unordered", csv.DictReader(open('mandelbrot_perfdata.csv')))
    x_ideal, y_ideal = (x_rayon, x_rayon)

    lines = [(x_rustspp, y_rustspp, "SSP-Rust Unordered", 's', '--'), 
             (x_rustspp_ordered, y_rustspp_ordered, "SSP-Rust Ordered", '3', '-.'), 
             (x_rayon, y_rayon, "Rayon", '4', '--'), 
             (x_tokio_ordered, y_tokio_ordered, "Tokio Ordered", '+', ':'), 
             (x_tokio_unordered, y_tokio_unordered, "Tokio Unordered", 'x', ':'),
             (x_ideal, y_ideal, "Ideal", '2', '-')]
   # print(lines)
    graph_multi(lines, 
        "Mandelbrot 1000x1000 Speedup", 
        "Speedup", 
        "Parallelism Level",
        "mandelbrot_comparison")


def plot_imageproc_speedup():
    x, y_rustspp = get_xy("threads_per_filter", "Speedup Rust-SPP", csv.DictReader(open('imageproc_perfdata.csv')))
    x, y_tokio = get_xy("threads_per_filter", "Speedup Tokio", csv.DictReader(open('imageproc_perfdata.csv')))
    x, y_tokio_normalized = get_xy("threads_per_filter", "Speedup Tokio", csv.DictReader(open('imageproc_perfdata.csv')))
   
    x = x[0:13]
    print(len(x))
    y_rustspp = y_rustspp[0:13]
    print(len(y_rustspp))
    y_tokio = y_tokio[0:13]
    print(len(y_tokio))
    y_tokio_normalized = y_tokio_normalized[1:13]
    y_tokio_normalized.append(y_tokio_normalized[-1])
    print(len(y_tokio_normalized))
    
    graph_multi([
                (x,y_rustspp, "SSP-Rust", "x", ":"), 
                (x,y_tokio, "Tokio", "x", "--"),
                (x,y_tokio_normalized, "Tokio Normalized", "o", ":")], 
        "Image processing speedup", 
        "Speedup", 
        "Parallelism Level",
        "imageproc_speedup", seq_label="Seq")

def plot_imageproc_throughput():
    x, y_rustspp = get_xy("threads_per_filter", "Throughput Rust-SPP", csv.DictReader(open('imageproc_perfdata.csv')))
    x, y_tokio = get_xy("threads_per_filter", "Throughput Tokio", csv.DictReader(open('imageproc_perfdata.csv')))
    
    graph_separated([
                (x,y_rustspp, "SSP-Rust", "x", "-", "Threads per stage"), 
                (x,y_tokio, "Tokio", "x", "-", "Buffer size per stage")],
        "Image processing throughput", 
        "Images/sec", 
        "imageproc_throughput", seq_label="Seq")


plot_mandelbrot_comparison()
plot_imageproc_throughput()
