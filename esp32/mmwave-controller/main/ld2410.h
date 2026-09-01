#ifndef LD2410_H
#define LD2410_H

#include <cstdint>
#include "driver/uart.h"

struct RadarTargetInfo {
    bool presence;
    bool moving_target;
    bool stationary_target;
    uint16_t moving_distance_cm;
    uint16_t stationary_distance_cm;
    uint16_t min_distance_cm;
};

class LD2410Sensor {
public:
    LD2410Sensor(uart_port_t uart_num, int tx_pin, int rx_pin);
    bool init(uint32_t baud_rate = 256000);
    bool readFrame(RadarTargetInfo& info);

private:
    uart_port_t m_uart_num;
    int m_tx_pin;
    int m_rx_pin;
    uint8_t m_buffer[256];
};

#endif // LD2410_H
