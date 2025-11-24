2025-11-17

# vtune self checker
```shell
/opt/intel/oneapi/vtune/2025.7/bin64/vtune-self-checker.sh
```

# vtune GPU profiling

```shell
# ajout de l'utilisateur au group 'render'
sudo usermod -aG render $USER

# [Profiling Hardware Without Intel Sampling Drivers](https://www.intel.com/content/www/us/en/docs/vtune-profiler/cookbook/2023-0/profiling-hardware-without-sampling-drivers.html)
sudo sh -c 'echo 0 > /proc/sys/kernel/perf_event_paranoid'

# 
╰─ source /opt/intel/oneapi/vtune/2025.7/vtune-vars.sh; vtune -collect gpu_offload -- ./target/profiling/fireworks_sim
vtune: Warning: To profile kernel modules during the session, make sure they are available in the /lib/modules/kernel_version/ location.
vtune: Collection started. To stop the collection, either press CTRL-C or enter from another console window: vtune -r $PATH_TO_PROJECT/r003go -command stop.
vtune: Collection stopped.
vtune: Using result path `$PATH_TO_PROJECT/r003go'
vtune: Executing actions 19 % Resolving information for `i915.ko'
vtune: Warning: Cannot locate debugging information for file `/lib/modules/6.1.0-32-amd64/kernel/drivers/gpu/drm/drm.ko'.
[...]
Collection and Platform Info
    Application Command Line: ./target/profiling/fireworks_sim
    Operating System: 6.1.0-32-amd64 12.12
    Computer Name: debian
    Result Size: 20.1 MB
    Collection start time: 12:44:17 17/11/2025 UTC
    Collection stop time: 12:44:19 17/11/2025 UTC
    Collector Type: Event-based sampling driver,Driverless Perf per-process sampling,User-mode sampling and tracing
    CPU
        Name: Intel(R) microarchitecture code named Tigerlake
        Frequency: 1.805 GHz
        Logical CPU Count: 8
    GPU
        Name: TigerLake-LP GT2 [Iris Xe Graphics]
        Vendor: Intel Corporation
        EU Count: 96
        Max EU Thread Count: 7
        Max Core Frequency: 1.350 GHz
        GPU OpenCL Info
            Version
            Max Compute Units
            Max Work Group Size
            Local Memory
            SVM Capabilities

Recommendations:
    GPU Time, % of Elapsed time: 43.7%
     | GPU utilization is low. Switch to the for in-depth analysis of host
     | activity. Poor GPU utilization can prevent the application from
     | offloading effectively.
    EU Array Stalled/Idle: 88.7% of Elapsed time with GPU busy
     | GPU metrics detect some kernel issues. Use GPU Compute/Media Hotspots
     | (preview) to understand how well your application runs on the specified
     | hardware.

If you want to skip descriptions of detected performance issues in the report,
enter: vtune -report summary -report-knob show-issues=false -r <my_result_dir>.
Alternatively, you may view the report in the csv format: vtune -report
<report_name> -format=csv.
vtune: Executing actions 100 % done
```
