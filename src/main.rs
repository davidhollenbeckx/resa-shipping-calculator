use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::{error::Error, fmt, fs, io, process};

// RATE DATA
// https://www.pirateship.com/usps/zone-map

fn zip_as_u32(zip_string: &str) -> Result<u32, Box<dyn Error>> {
    if zip_string.is_empty() {
        return Err(Box::new(UnexpectedError {
            message: "No zip_string.".to_string(),
        }));
    }

    let first_part = zip_string.split('-').next().unwrap_or("");

    return first_part
        .parse::<u32>()
        .map_err(|e| Box::new(e) as Box<dyn Error>);
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
enum ShippingMethod {
    Economy,
    Ground,
    Express,
    Expedited,
    Priority,
    Special,
    Unknown,
}

impl ShippingMethod {
    fn from_str(str: &str) -> ShippingMethod {
        match str {
            // economy
            "UPS Worldwide Saver (Duties Not Paid)" => ShippingMethod::Economy,
            "UPS SurePost" => ShippingMethod::Economy,
            "UPS SUREPOST" => ShippingMethod::Economy,
            "DHL International (Duties Not Paid)" => ShippingMethod::Economy,
            "DHL International [Route Protection Highly Recommended Not Responsible For Lost Shipment]" => ShippingMethod::Economy,
            "USPS First-Class Mail [Order Protection Highly Recommended Not Responsible For Lost Shipment]" => ShippingMethod::Economy,
            "USPS First-Class Mail" => ShippingMethod::Economy,
            "DHL eCommerce Ground" => ShippingMethod::Economy,
            "USPS Parcel Post" => ShippingMethod::Economy,

            // ground


            "UPS Ground [RESA]" => ShippingMethod::Ground,
            "FedEx Home Delivery" => ShippingMethod::Ground,
            "FedEx Ground" => ShippingMethod::Ground,
            "Upgrade to (3-5 Day) DHL Expedited" => ShippingMethod::Ground,

            // expedited
            "UPS Next Day Air Saver" => ShippingMethod::Expedited,
            "FedEx Standard Overnight [RESA]" => ShippingMethod::Expedited,
            "FedEx Priority Overnight" => ShippingMethod::Expedited,
            "FedEx Standard Overnight (Envelope)" => ShippingMethod::Expedited,
            "USPS Express Mail" => ShippingMethod::Expedited,
            "FedEx Standard Overnight" => ShippingMethod::Expedited,


            // express

            "UPS Worldwide Express (Duties Not Paid)" => ShippingMethod::Express,
            "FedEx One Rate (Pak) 2-Day [RESA]" => ShippingMethod::Express,
            "FedEx One Rate (Envelope) 2-Day [RESA]" => ShippingMethod::Express,
            "UPS 2nd Day Air" => ShippingMethod::Express,
            "FedEx 2nd Day [RESA]" => ShippingMethod::Express,
            "USPS Priority Mail" => ShippingMethod::Express,




            // priority

            "FedEx Intl Priority (Envelope) (Duties Not Paid)" => ShippingMethod::Priority,
            "USPS Priority Mail International (Duties Not Paid)" => ShippingMethod::Priority,
            "FedEx Intl Connect Plus (Duties Not Paid) [RESA]" => ShippingMethod::Priority,
            "FedEx 2nd Day" => ShippingMethod::Priority,
            "FedEx International Priority (Duties Not Paid)" => ShippingMethod::Priority,
            "fedex overnight" => ShippingMethod::Priority,
            "FEDEX overnight" => ShippingMethod::Priority,

            // special cases - jewelry or transfer
            "FedEx One Rate (Pak) 2-Day [RESA JEWELRY]" => ShippingMethod::Special,
            "Misc Transfer Carrier" => ShippingMethod::Special,

            // these ones are sus - found international stuff etc. maybe ground?
            "USPS" => ShippingMethod::Unknown,
            "fedex" => ShippingMethod::Unknown,
            "FEDEX" => ShippingMethod::Unknown,
            "ups" => ShippingMethod::Unknown,
            "UPS" => ShippingMethod::Unknown,
            "usps" => ShippingMethod::Unknown,
            "FEDEx" => ShippingMethod::Unknown,

            _ => ShippingMethod::Unknown,
        }
    }
    fn name(&self) -> String {
        match self {
            ShippingMethod::Economy => String::from("Economy"),
            ShippingMethod::Ground => String::from("Ground"),
            ShippingMethod::Express => String::from("Express"),
            ShippingMethod::Expedited => String::from("Expedited"),
            ShippingMethod::Priority => String::from("Priority"),
            ShippingMethod::Special => String::from("Special"),
            ShippingMethod::Unknown => String::from("Unknown"),
        }
    }
}

enum Province {
    AK,
    AL,
    AR,
    AZ,
    CA,
    CO,
    CT,
    DC,
    DE,
    FL,
    GA,
    HI,
    IA,
    ID,
    IL,
    IN,
    KS,
    KY,
    LA,
    MA,
    MD,
    ME,
    MI,
    MN,
    MO,
    MS,
    MT,
    NC,
    ND,
    NE,
    NH,
    NJ,
    NM,
    NV,
    NY,
    OH,
    OK,
    OR,
    PA,
    PR,
    RI,
    SC,
    SD,
    TN,
    TX,
    UT,
    VA,
    VT,
    WA,
    WI,
    WV,
    WY,
}

impl Province {
    fn from_zip_code(zip_code: u32) -> Result<Province, Box<dyn Error>> {
        match zip_code {
            20042..=20042 => Ok(Province::VA),
            20331..=20331 => Ok(Province::MD),
            99501..=99950 => Ok(Province::AK),
            35004..=36925 => Ok(Province::AL),
            71601..=72959 => Ok(Province::AR),
            75502..=75502 => Ok(Province::AR),
            85001..=86556 => Ok(Province::AZ),
            90001..=96162 => Ok(Province::CA),
            80001..=81658 => Ok(Province::CO),
            6001..=6389 => Ok(Province::CT),
            6401..=6928 => Ok(Province::CT),
            20001..=20039 => Ok(Province::DC),
            20042..=20599 => Ok(Province::DC),
            20799..=20799 => Ok(Province::DC),
            19701..=19980 => Ok(Province::DE),
            32004..=34997 => Ok(Province::FL),
            30001..=31999 => Ok(Province::GA),
            39901..=39901 => Ok(Province::GA),
            96701..=96898 => Ok(Province::HI),
            50001..=52809 => Ok(Province::IA),
            68119..=68120 => Ok(Province::IA),
            83201..=83876 => Ok(Province::ID),
            60001..=62999 => Ok(Province::IL),
            46001..=47997 => Ok(Province::IN),
            66002..=67954 => Ok(Province::KS),
            40003..=42788 => Ok(Province::KY),
            70001..=71232 => Ok(Province::LA),
            71234..=71497 => Ok(Province::LA),
            1001..=2791 => Ok(Province::MA),
            5501..=5544 => Ok(Province::MA),
            20335..=20797 => Ok(Province::MD),
            20812..=21930 => Ok(Province::MD),
            3901..=4992 => Ok(Province::ME),
            48001..=49971 => Ok(Province::MI),
            55001..=56763 => Ok(Province::MN),
            63001..=65899 => Ok(Province::MO),
            38601..=39776 => Ok(Province::MS),
            71233..=71233 => Ok(Province::MS),
            59001..=59937 => Ok(Province::MT),
            27006..=28909 => Ok(Province::NC),
            58001..=58856 => Ok(Province::ND),
            68001..=68118 => Ok(Province::NE),
            68122..=69367 => Ok(Province::NE),
            3031..=3897 => Ok(Province::NH),
            7001..=8989 => Ok(Province::NJ),
            87001..=88441 => Ok(Province::NM),
            88901..=89883 => Ok(Province::NV),
            6390..=6390 => Ok(Province::NY),
            10001..=14975 => Ok(Province::NY),
            43001..=45999 => Ok(Province::OH),
            73001..=73199 => Ok(Province::OK),
            73401..=74966 => Ok(Province::OK),
            97001..=97920 => Ok(Province::OR),
            15001..=19640 => Ok(Province::PA),
            00600..=00799 => Ok(Province::PR),
            00900..=00999 => Ok(Province::PR),
            2801..=2940 => Ok(Province::RI),
            29001..=29948 => Ok(Province::SC),
            57001..=57799 => Ok(Province::SD),
            37010..=38589 => Ok(Province::TN),
            73301..=73301 => Ok(Province::TX),
            75001..=75501 => Ok(Province::TX),
            75503..=79999 => Ok(Province::TX),
            88510..=88589 => Ok(Province::TX),
            84001..=84784 => Ok(Province::UT),
            20040..=20041 => Ok(Province::VA),
            20040..=20167 => Ok(Province::VA),
            22001..=24658 => Ok(Province::VA),
            5001..=5495 => Ok(Province::VT),
            5601..=5907 => Ok(Province::VT),
            98001..=99403 => Ok(Province::WA),
            53001..=54990 => Ok(Province::WI),
            24701..=26886 => Ok(Province::WV),
            82001..=83128 => Ok(Province::WY),
            _ => Err(Box::new(UnexpectedError {
                message: "Invalude Zip Range.".to_string(),
            })),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
enum Region {
    All,
    Northeast,
    MidAtlantic,
    Southeast,
    Midwest,
    GreatPlains,
    Southwest,
    Mountain,
    WestCoast,
    PuertoRico,
    Hawaii,
    Alaska,
    International,
}

impl Region {
    fn from_string_zip(zip_string: &str) -> Region {
        // 20_000 is higher than any us zip ID
        let zip = zip_as_u32(zip_string).unwrap_or(20_000);
        let province = Province::from_zip_code(zip);
        if let Ok(province_value) = province {
            return Region::from_province(province_value);
        } else {
            Region::International
        }
    }

    fn from_province(province: Province) -> Region {
        use Province::*;
        match province {
            ME | NH | VT | MA | RI | CT | NY | NJ | PA => Region::Northeast,
            DC | DE | MD | VA | WV | NC => Region::MidAtlantic,
            KY | LA | AR | SC | GA | FL | AL | MS | TN => Region::Southeast,
            OH | MI | IN | IL | WI | MN | IA | MO => Region::Midwest,
            ND | SD | NE | KS | OK => Region::GreatPlains,
            TX | NM | AZ => Region::Southwest,
            CO | WY | MT | ID | UT | NV => Region::Mountain,
            CA | OR | WA => Region::WestCoast,
            PR => Region::PuertoRico,
            HI => Region::Hawaii,
            AK => Region::Alaska,
        }
    }

    fn name(&self) -> String {
        match self {
            Region::Northeast => String::from("Northeast"),
            Region::MidAtlantic => String::from("Mid-Atlantic"),
            Region::Southeast => String::from("Southeast"),
            Region::Midwest => String::from("Midwest"),
            Region::GreatPlains => String::from("Great Plains"),
            Region::Southwest => String::from("Southwest"),
            Region::Mountain => String::from("Mountain"),
            Region::WestCoast => String::from("West Coast"),
            Region::PuertoRico => String::from("Puerto Rico"),
            Region::Hawaii => String::from("Hawaii"),
            Region::Alaska => String::from("Alaska"),
            Region::International => String::from("International"),
            Region::All => String::from("All Regions"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
enum WeightRange {
    Under2Lbs,
    Between2And5Lbs,
    Over5Lbs,
    Unknown,
}

impl WeightRange {
    fn from_str(weight_str: &str) -> WeightRange {
        let weight_result = weight_str.parse::<f32>();
        return if let Ok(weight_as_f32) = weight_result {
            if weight_as_f32 < 2.0 {
                return WeightRange::Under2Lbs;
            }
            if weight_as_f32 < 5.0 {
                return WeightRange::Between2And5Lbs;
            }
            WeightRange::Over5Lbs
        } else {
            WeightRange::Unknown
        };
    }
    fn name(&self) -> String {
        match self {
            WeightRange::Under2Lbs => String::from("Orders under 2 Pounds"),
            WeightRange::Between2And5Lbs => String::from("Orders between 2 and 5 pounds"),
            WeightRange::Over5Lbs => String::from("Orders over 5 Pounds"),
            WeightRange::Unknown => String::from("Orders where weight is not known"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct OrderFromCSVInput {
    zip: String,
    retail_value: String,
    ship_weight: String,
    shipping_method: String,
    label_cost: String,
    packaging_cost: String,
    labor_cost: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Order {
    ship_weight: WeightRange,
    retail_value: f32,
    shipping_cost: f32,
    shipping_cost_per_pound: f32,
    shipping_method: ShippingMethod,
    region: Region,
}

impl Order {
    fn new_from_csv(input: &OrderFromCSVInput) -> Option<Order> {
        let ship_weight = WeightRange::from_str(&input.ship_weight);
        let ship_weight_f32 = input.ship_weight.parse::<f32>().ok()?;
        let region = Region::from_string_zip(&input.zip);
        let retail_value = input.retail_value.parse::<f32>().unwrap_or(0.0);
        let shipping_method = ShippingMethod::from_str(&input.shipping_method);

        let label_cost = input.label_cost.parse::<f32>().unwrap_or(0.0);
        let packaging_cost = input.packaging_cost.parse::<f32>().unwrap_or(0.0);
        let labor_cost = input.labor_cost.parse::<f32>().unwrap_or(0.0);
        let shipping_cost = labor_cost + label_cost + packaging_cost;
        let shipping_cost_per_pound = shipping_cost / ship_weight_f32;

        return Some(Order {
            ship_weight,
            region,
            shipping_cost,
            shipping_cost_per_pound,
            retail_value,
            shipping_method,
        });
    }
}

// FILE OPS

fn append_line_to_file(file_path: &str, line: String) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file_path)?;

    writeln!(file, "{}", line)?;

    Ok(())
}

fn write_to_output_file(data: &Vec<Order>) -> Result<(), Box<dyn Error>> {
    let json_string = serde_json::to_string_pretty(&data)?;
    let mut file = fs::File::create("output.json")?;
    file.write_all(json_string.as_bytes())?;
    Ok(())
}

fn write_to_error_file(data: &Vec<OrderFromCSVInput>) -> Result<(), Box<dyn Error>> {
    let json_string = serde_json::to_string_pretty(&data)?;
    let mut file = fs::File::create("errors.json")?;
    file.write_all(json_string.as_bytes())?;
    Ok(())
}

fn write_avgs_to_output_file(data: &Vec<AverageOutput>) -> Result<(), Box<dyn Error>> {
    let json_string = serde_json::to_string_pretty(&data)?;
    let mut file = fs::File::create("avg_output.json")?;
    file.write_all(json_string.as_bytes())?;
    Ok(())
}

fn write_to_csv(avgs: &[AverageOutput]) -> Result<(), Box<dyn Error>> {
    let file = fs::File::create("output.csv")?;
    let mut wtr = csv::Writer::from_writer(file);

    for record in avgs {
        wtr.serialize(record)?;
    }

    wtr.flush()?;
    Ok(())
}

// ERROR HANDLING

#[derive(Debug)]
struct UnexpectedError {
    message: String,
}

impl fmt::Display for UnexpectedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unexpected Error: {}", self.message)
    }
}

impl Error for UnexpectedError {}

// CSV PARSE FUNCTION

struct Counter {
    total_retail_cost: f32,
    total_item_count: f32,
    total_shipping_cost: f32,
    total_shipping_cost_per_pound: f32,
}

impl Counter {
    fn new() -> Self {
        Counter {
            total_retail_cost: 0.0,
            total_item_count: 0.0,
            total_shipping_cost: 0.0,
            total_shipping_cost_per_pound: 0.0,
        }
    }

    fn update(&mut self, retail_cost: f32, shipping_cost: f32, shipping_cost_per_pound: f32) {
        self.total_retail_cost += retail_cost;
        self.total_item_count += 1.0;
        self.total_shipping_cost += shipping_cost;
        self.total_shipping_cost_per_pound += shipping_cost_per_pound;
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct AverageOutput {
    region: Region,
    label: String,
    shipping_method: String,
    avg: f32,
}

fn report_shipping_method(shipping_method: &ShippingMethod) -> bool {
    match shipping_method {
        ShippingMethod::Economy => true,
        ShippingMethod::Ground => true,
        _ => false
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let file_path = "input.csv";
    let file = fs::File::open(file_path)?;

    let mut zip_code_index: Option<usize> = None;
    let mut shipping_method_index: Option<usize> = None;
    let mut label_cost_index: Option<usize> = None;
    let mut packaging_cost_index: Option<usize> = None;
    let mut labor_cost_index: Option<usize> = None;
    let mut ship_weight_index: Option<usize> = None;
    let mut retail_value_index: Option<usize> = None;

    let mut rdr = csv::Reader::from_reader(file);
    {
        // We nest this call in its own scope because of lifetimes.
        let headers = rdr.headers()?;
        for (index, header_string) in headers.iter().enumerate() {
            match header_string {
                "Labor (Pick/Pack) Spend" => labor_cost_index = Some(index),
                "Carrier Service" => shipping_method_index = Some(index),
                "Label (Carrier) Spend" => label_cost_index = Some(index),
                "Weight of Units Shipped (lbs)" => ship_weight_index = Some(index),
                "Material (Packaging) Spend" => packaging_cost_index = Some(index),
                "Retail Value (Ref)" => retail_value_index = Some(index),
                "Recipient Zip" => zip_code_index = Some(index),
                _ => {}
            }
        }
    }

    let zip_code_index = zip_code_index.unwrap();
    let ship_weight_index = ship_weight_index.unwrap();
    let shipping_method_index = shipping_method_index.unwrap();
    let label_cost_index = label_cost_index.unwrap();
    let packaging_cost_index = packaging_cost_index.unwrap();
    let labor_cost_index = labor_cost_index.unwrap();
    let retail_value_index = retail_value_index.unwrap();

    let mut parsed_orders: Vec<Order> = vec![];
    let mut errors: Vec<OrderFromCSVInput> = vec![];

    for result in rdr.records() {
        let record = result?;

        let order_from_csv_input = OrderFromCSVInput {
            zip: record.get(zip_code_index).unwrap().to_owned(),
            retail_value: record.get(retail_value_index).unwrap_or("").to_owned(),
            ship_weight: record.get(ship_weight_index).unwrap().to_owned(),
            shipping_method: record.get(shipping_method_index).unwrap().to_owned(),
            label_cost: record.get(label_cost_index).unwrap_or("").to_owned(),
            packaging_cost: record.get(packaging_cost_index).unwrap_or("").to_owned(),
            labor_cost: record.get(labor_cost_index).unwrap_or("").to_owned(),
        };

        let order = Order::new_from_csv(&order_from_csv_input);

        if let Some(order_value) = order {
            parsed_orders.push(order_value);
        } else {
            errors.push(order_from_csv_input);
        }
    }
    write_to_output_file(&parsed_orders)?;
    write_to_error_file(&errors)?;

    let mut cost_rate_counter: HashMap<(Region, ShippingMethod), Counter> = HashMap::new();
    let mut per_pound_rate_counter: HashMap<(Region, ShippingMethod), Counter> = HashMap::new();
    let mut shipping_rate_counter: HashMap<(Region, WeightRange, ShippingMethod), Counter> =
        HashMap::new();
    for order in parsed_orders {
        cost_rate_counter
            .entry((order.region.clone(), order.shipping_method.clone()))
            .and_modify(|counter| {
                counter.update(
                    order.retail_value,
                    order.shipping_cost,
                    order.shipping_cost_per_pound,
                )
            })
            .or_insert_with(|| Counter::new());

        per_pound_rate_counter
            .entry((order.region.clone(), order.shipping_method.clone()))
            .and_modify(|counter| {
                counter.update(
                    order.retail_value,
                    order.shipping_cost,
                    order.shipping_cost_per_pound,
                )
            })
            .or_insert_with(|| Counter::new());

        shipping_rate_counter
            .entry((
                order.region.clone(),
                order.ship_weight.clone(),
                order.shipping_method.clone(),
            ))
            .and_modify(|counter| {
                counter.update(
                    order.retail_value,
                    order.shipping_cost,
                    order.shipping_cost_per_pound,
                )
            })
            .or_insert_with(|| Counter::new());

        if order.region == Region::International
            || order.region == Region::Alaska
            || order.region == Region::Hawaii
        {
            continue;
        }

        // Region::All is for continental US

        per_pound_rate_counter
            .entry((Region::All, order.shipping_method.clone()))
            .and_modify(|counter| {
                counter.update(
                    order.retail_value,
                    order.shipping_cost,
                    order.shipping_cost_per_pound,
                )
            })
            .or_insert_with(|| Counter::new());

        cost_rate_counter
            .entry((Region::All, order.shipping_method.clone()))
            .and_modify(|counter| {
                counter.update(
                    order.retail_value,
                    order.shipping_cost,
                    order.shipping_cost_per_pound,
                )
            })
            .or_insert_with(|| Counter::new());

        shipping_rate_counter
            .entry((Region::All, order.ship_weight, order.shipping_method))
            .and_modify(|counter| {
                counter.update(
                    order.retail_value,
                    order.shipping_cost,
                    order.shipping_cost_per_pound,
                )
            })
            .or_insert_with(|| Counter::new());
    }

    let mut cost_rate_avg: HashMap<(Region, ShippingMethod), f32> = HashMap::new();
    let mut per_pound_rate_avg: HashMap<(Region, ShippingMethod), f32> = HashMap::new();
    let mut shipping_rate_avg: HashMap<(Region, WeightRange, ShippingMethod), f32> = HashMap::new();

    for (key, counter) in cost_rate_counter {
        let avg_retail_cost = counter.total_retail_cost / counter.total_item_count;
        let avg_shipping_cost = counter.total_shipping_cost / counter.total_item_count;
        let cost_per_dollar = avg_shipping_cost / avg_retail_cost;

        cost_rate_avg.insert(key, cost_per_dollar);
    }

    for (key, counter) in per_pound_rate_counter {
        let cost_per_pound = counter.total_shipping_cost_per_pound / counter.total_item_count;
        per_pound_rate_avg.insert(key, cost_per_pound);
    }

    for (key, counter) in shipping_rate_counter {
        let avg_shipping_cost = counter.total_shipping_cost / counter.total_item_count;
        shipping_rate_avg.insert(key, avg_shipping_cost);
    }

    let mut avgs: Vec<AverageOutput> = vec![];

    for ((region, shipping_method), avg) in cost_rate_avg {
        if !report_shipping_method(&shipping_method) {
            continue
        }
        avgs.push(AverageOutput {
            region,
            shipping_method: shipping_method.name(),
            label: "Cost per $".to_string(),
            avg,
        })
    }

    for ((region, shipping_method), avg) in per_pound_rate_avg {
        if !report_shipping_method(&shipping_method) {
            continue
        }
        avgs.push(AverageOutput {
            region,
            shipping_method: shipping_method.name(),
            label: "$ per Pound".to_string(),
            avg,
        })
    }

    for ((region, weight_range, shipping_method), avg) in shipping_rate_avg {
        if !report_shipping_method(&shipping_method) {
            continue
        }
        avgs.push(AverageOutput {
            region,
            shipping_method: shipping_method.name(),
            label: weight_range.name(),
            avg,
        })
    }



    avgs.sort_by_key(|k| k.region.name());

    write_avgs_to_output_file(&avgs)?;
    write_to_csv(&avgs)?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let result = run();
    match result.await {
        Ok(result) => (),
        Err(err) => eprintln!("Error: {:?}", err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_to_region_standard() {
        let region = Region::from_string_zip("10016").ok();
        assert_eq!(region, Some(Region::Northeast));
    }

    #[test]
    fn test_invalid_zips() {
        let region_1 = Region::from_string_zip("203000-");
        let region_2 = Region::from_string_zip("");
        assert!(!region_1.is_ok());
        assert!(!region_2.is_ok());
    }

    #[test]
    fn test_split_zips() {
        let region_1 = Region::from_string_zip("20044-2932").ok();
        let region_2 = Region::from_string_zip("95060-9412").ok();
        assert_eq!(region_1, Some(Region::MidAtlantic));
        assert_eq!(region_2, Some(Region::WestCoast));
    }

    // You can add more tests here
}
