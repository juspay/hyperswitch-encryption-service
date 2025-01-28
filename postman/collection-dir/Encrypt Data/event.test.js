pm.test("Decrypt Data - Status code is 200", function () {
    pm.response.to.have.status(200);
});
var response = pm.response.json();

pm.environment.set("value", response.data.value);
