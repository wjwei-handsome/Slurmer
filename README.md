# slurmer

A TUI application for monitoring and managing SLURM jobs.

It provides an intuitive, interactive interface to view, filter, sort, and manage SLURM jobs, making SLURM job management more efficient and user-friendly.

## ✨ Features

- **🔄 Real-time Job Monitoring**: View and refresh SLURM job statuses in real-time
![](./images/monitor.png)
- **🔍 Advanced Filtering**: Filter jobs by user, state, partition, QoS, job name, and more in real-time(regex supported)
![](./images/filter.png)
- **📊 Customizable Columns**: Flexibly configure which job information columns to display and in what order
![](./images/columns.png)
- **📝 Job Details View**: Examine job scripts and job logs
![](./images/script.png)<br>![](./images/log.png)
- **🎮 Job Management**: Cancel selected jobs
![](./images/cancel.png)
- **📈 Job History & Efficiency**: Review past jobs from the Slurm accounting database and see how much of the CPU, memory and walltime they reserved was actually used, so over-allocated jobs are easy to spot
![](./images/history.png)<br>![](./images/history-cpu.png)

  - **CPU%** — `TotalCPU / (AllocCPUS x Elapsed)`: how much of the reserved CPU time the job burned, the same ratio `seff` reports
  - **Mem%** — peak `MaxRSS` over the memory allocated per node
  - **Time%** — elapsed time over the requested time limit

  Running and pending jobs are listed but show `-`, since Slurm only records their CPU and memory usage once their steps finish.

  The history view reuses whatever is set in the filter menu (user, state, partition, QoS, job name and node regexes), and <kbd>←</kbd>/<kbd>→</kbd> moves the time window between 24h, 3d, 7d, 30d and 90d.


<!-- | 🔄 **Real-time Job Monitoring** | 🔍 **Advanced Filtering** | 📊 **Customizable Columns** |
|----------------------------------|---------------------------|------------------------------|
| **View and refresh SLURM job statuses in real-time**<br>![](./images/monitor.png)       | **Filter jobs by user, state, partition, QoS, job name, and more in real-time (regex supported)**<br>![](./images/filter.png)  | **Flexibly configure which job information columns to display and in what order**<br>![](./images/columns.png)    |

| 📝 **Job Details View**         | 🎮 **Job Management**     |                              |
|----------------------------------|---------------------------|------------------------------|
| **Examine job scripts and job logs**<br>![](./images/script.png)<br>![](./images/log.png) | **Cancel selected jobs directly from the UI**<br>![](./images/cancel.png) |                              | -->

## 🛠️ Installation

```bash
cargo install slurmer
```
or install from the latest source code:

```bash
cargo install --git https://github.com/wjwei-handsome/Slurmer.git
```


## 📖 Usage

Just run `slurmer`.

## ⌨️ Keyboard Shortcuts

- <kbd>↓/↑</kbd>: Move up and down in the job list
- <kbd>Shift + ↓/↑</kbd>: Move job in the log-view/script-view
- <kbd>f</kbd>: Open filter menu
- <kbd>c</kbd>: Open column selection menu
- <kbd>v</kbd>: View job logs
- <kbd>h</kbd>: Job history with CPU/memory/walltime efficiency
- <kbd>Enter</kbd>: View job script
- <kbd>Space</kbd>: Select job
- <kbd>a</kbd>: Select all jobs
- <kbd>r</kbd>: Refresh job list
- <kbd>x</kbd>: Cancel selected jobs
- <kbd>Esc</kbd>: Quit application

More detailed keybindings can be found each popup menu.

## 🔗 Dependencies

- slurm utilities (e.g., `squeue`, `scancel`) is required.
- `sacct` and a cluster running Slurm job accounting are required for the job history view.
- [`bat`](https://github.com/sharkdp/bat) is optional for viewing job scripts.

## ⚙️ Configuration

`slurmer` automatically detects available SLURM partitions and QoS in your system and uses the currently logged-in username as the default filter.

## 👥 Contributing

Contributions are welcome! Feel free to submit issues or pull requests.

## 📜 License

Copyright (c) wjwei-handsome <weiwenjie@westlake.edu.cn>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
