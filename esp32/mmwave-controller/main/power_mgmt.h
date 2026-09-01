#ifndef POWER_MGMT_H
#define POWER_MGMT_H

#include "driver/gpio.h"

class PowerManagerMCU {
public:
    PowerManagerMCU(gpio_num_t wake_pin);
    void init();
    void setWakeSignal(bool active);
    void updatePresenceState(bool presence_in_range, uint32_t hold_time_ms = 5000);

private:
    gpio_num_t m_wake_pin;
    bool m_wake_active;
    uint64_t m_last_presence_time_ms;
};

#endif // POWER_MGMT_H
