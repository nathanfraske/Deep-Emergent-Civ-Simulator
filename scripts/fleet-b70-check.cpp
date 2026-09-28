// Copyright 2026 Nathan M. Fraske
// Licensed under the Apache License, Version 2.0; see LICENSE.
// Infrastructure test fixture, independent of simulation calibration.
#include <sycl/sycl.hpp>
#include <algorithm>
#include <cstdint>
#include <iomanip>
#include <iostream>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

int main(int argc, char **argv) {
  try {
    if (argc != 2) throw std::runtime_error("Expected comma-separated PCI addresses");
    std::set<std::string> wanted;
    std::stringstream input(argv[1]);
    for (std::string pci; std::getline(input, pci, ',');) wanted.insert(pci);
    if (wanted.empty() || wanted.size() > 2) throw std::runtime_error("Invalid device count");
    std::vector<sycl::device> devices;
    std::set<std::string> found;
    for (const auto &device : sycl::device::get_devices(sycl::info::device_type::gpu)) {
      if (device.get_backend() != sycl::backend::ext_oneapi_level_zero) continue;
      if (!device.has(sycl::aspect::ext_intel_pci_address))
        throw std::runtime_error("PCI identity unavailable");
      auto pci = device.get_info<sycl::ext::intel::info::device::pci_address>();
      auto name = device.get_info<sycl::info::device::name>();
      if (!wanted.count(pci) || !found.insert(pci).second || name.find("B70") == std::string::npos)
        throw std::runtime_error("Unexpected or duplicate GPU");
      devices.push_back(device);
    }
    if (found != wanted) throw std::runtime_error("Selected GPUs not available");

    constexpr std::size_t count = 262147, guards = 8;
    constexpr std::uint32_t sentinel = 0xc0ffee12u;
    std::vector<std::uint32_t> reference(count);
    for (std::size_t i = 0; i < count; ++i) {
      std::uint32_t x = static_cast<std::uint32_t>(i) ^ 0x9e3779b9u;
      for (unsigned step = 0; step < 64; ++step) {
        x = x * 1664525u + 1013904223u;
        x ^= x >> 13;
      }
      reference[i] = x;
    }
    std::vector<sycl::queue> queues;
    std::vector<std::uint32_t *> buffers;
    for (const auto &device : devices) {
      queues.emplace_back(device, sycl::property::queue::in_order{});
      auto &queue = queues.back();
      auto *values = sycl::malloc_shared<std::uint32_t>(count + guards, queue);
      if (!values) throw std::runtime_error("USM allocation failed");
      std::fill(values, values + count + guards, sentinel);
      buffers.push_back(values);
    }
    // Submit work to every selected device before waiting for any device.
    for (std::size_t d = 0; d < devices.size(); ++d) {
      auto *values = buffers[d];
      queues[d].parallel_for(sycl::range<1>(count), [=](sycl::id<1> id) {
        std::uint32_t x = static_cast<std::uint32_t>(id[0]) ^ 0x9e3779b9u;
        for (unsigned step = 0; step < 64; ++step) {
          x = x * 1664525u + 1013904223u;
          x ^= x >> 13;
        }
        values[id[0]] = x;
      });
    }
    for (auto &queue : queues) queue.wait_and_throw();
    std::ostringstream result;
    result << "{\"passed\":true,\"elements_per_device\":" << count
           << ",\"iterations_per_element\":64,\"devices\":[";
    for (std::size_t d = 0; d < devices.size(); ++d) {
      std::size_t mismatches = 0;
      bool intact = true;
      for (std::size_t i = 0; i < count; ++i) mismatches += buffers[d][i] != reference[i];
      for (std::size_t i = count; i < count + guards; ++i) intact &= buffers[d][i] == sentinel;
      sycl::free(buffers[d], queues[d]);
      if (mismatches || !intact) throw std::runtime_error("Numerical or guard mismatch");
      if (d) result << ',';
      result << "{\"pci\":" << std::quoted(devices[d].get_info<sycl::ext::intel::info::device::pci_address>())
             << ",\"name\":" << std::quoted(devices[d].get_info<sycl::info::device::name>())
             << ",\"backend\":\"level_zero\",\"mismatches\":0,\"guard_unchanged\":true}";
    }
    result << "]}";
    std::cout << result.str() << '\n';
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
