pm.test("Decrypt Data - Status code is 200", function () {
    pm.response.to.have.status(200);
});
var response = pm.response.json();

pm.environment.set("value1", response.data[0].value1);
pm.environment.set("value2", response.data[0].value2);
pm.environment.set("value3", response.data[1].value3);
pm.environment.set("value4", response.data[1].value4);
